use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use log::{error, info, trace};
use midir::{MidiInput, MidiInputPort, MidiOutput, MidiOutputConnection, os::unix::VirtualOutput};

struct OutState {
    out: MidiOutputConnection,
    hihat_pressed: bool,
    double_pedal: bool,
    mappings: Vec<(u8, u8)>,
}

fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let matches = clap::Command::new("alesis_hihat")
        .arg(
            clap::Arg::new("port")
                .short('p')
                .help("midi port to connect, determine automatically if not specified"),
        )
        .arg(
            clap::Arg::new("out")
                .short('o')
                .default_value("alesis_hihat"),
        )
        .arg(clap::Arg::new("map").short('m').num_args(0..))
        .arg(
            clap::Arg::new("list")
                .short('l')
                .help("list all inputs")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("double-pedal")
                .short('d')
                .help("double-pedal mode")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let midi_in = MidiInput::new("alesis_hihat").context("initialising midi input")?;
    let midi_out = MidiOutput::new("alesis_hihat").context("initializing midi output")?;
    let out_name = matches.get_one::<String>("out").unwrap();

    let mut mappings: Vec<(u8, u8)> = Vec::new();
    if let Some(matches) = matches.get_many::<String>("map") {
        for m in matches {
            let Some((from, to)) = m.split_once(":") else {
                return Err(anyhow!("mappings shall be in format 'from:to'"));
            };
            let (Ok(from), Ok(to)) = (from.parse(), to.parse()) else {
                return Err(anyhow!("notes must be numbers"));
            };
            mappings.push((from, to));
        }
    }

    let out_port = match MidiOutput::create_virtual(midi_out, out_name) {
        Err(e) => {
            return Err(anyhow!("creating virtual output port: {e}"));
        }
        Ok(v) => v,
    };
    let mut in_port: Option<MidiInputPort> = None;
    let state = OutState {
        out: out_port,
        hihat_pressed: false,
        double_pedal: matches.get_flag("double-pedal"),
        mappings,
    };

    if matches.get_flag("list") {
        for p in midi_in.ports() {
            println!("{} => {}", p.id(), midi_in.port_name(&p).unwrap());
        }
        return Ok(());
    }
    if let Some(id) = matches.get_one::<String>("port") {
        in_port = Some(
            midi_in
                .find_port_by_id(id.clone())
                .context("getting port by id")?,
        );
    } else {
        for p in midi_in.ports() {
            let port_name = midi_in.port_name(&p).context("getting port name")?;
            if port_name.contains("Alesis") && port_name.contains("MIDI") {
                info!("Automatically selecting port {}: {}", p.id(), port_name);
                in_port = Some(p);
                break;
            }
        }
    }
    if in_port.is_none() {
        return Err(anyhow!("no port found"));
    }

    let in_port = in_port.unwrap();
    info!(
        "Reading from port {}, writing to port '{}', press Ctrl+C to exit",
        in_port.id(),
        out_name
    );

    match midi_in.connect(&in_port, "midi test", handle_midi_data, state) {
        Err(e) => {
            Err(anyhow!("connecting to midi input: {e}"))?;
        }
        Ok(_c) => loop {
            sleep(Duration::from_millis(300));
        },
    };
    Ok(())
}

fn handle_midi_data(ts: u64, message: &[u8], state: &mut OutState) {
    let start = Instant::now();
    if message[0] != 0xf8 {
        // skip clock messages
        trace!("Raw MIDI input: ts {ts}: {message:?}");
    }

    let out_msg = match message {
        [0xb9, 0x04, 0x00] => {
            // control change indicating the next note will be open hihat
            state.hihat_pressed = false;
            Some(&[0xb9, 0x04, 0x00])
        }
        [0xb9, 0x04, 0x7f] => {
            // control change indicating the next note will be closed hihat
            state.hihat_pressed = true;
            Some(&[0xb9, 0x04, 0x7f])
        }
        [0xa9, 0x2e, pressure] => {
            if !state.double_pedal {
                // supress "polyphonic aftertouch" for hi-hat notes if we have double pedal mode enabled
                // otherwise it will mute our hi-hats
                Some(&[0xa9, 0x2e, *pressure])
            } else {
                None
            }
        }
        [0x99, 0x2c, vel] => {
            // hi-hat pedal note, replace with kick if double pedal mode is enabled
            if state.double_pedal {
                Some(&[0x99, 0x24, *vel])
            } else {
                Some(&[0x99, 0x2c, *vel])
            }
        }
        [0x99, 0x2e, vel] => {
            // hi-hat note
            if state.hihat_pressed && !state.double_pedal {
                // replace note 46 (open hihat) to 42 (closed hihat)
                Some(&[0x99, 0x2a, *vel])
            } else {
                Some(&[0x99, 0x2e, *vel])
            }
        }
        [0x99, note, vel] => {
            if let Some(new_note) = state
                .mappings
                .iter()
                .find_map(|(from, to)| if note == from { Some(to) } else { None })
            {
                Some(&[0x99, *new_note, *vel])
            } else {
                Some(&[0x99, *note, *vel])
            }
        }
        [d1, d2, d3] => Some(&[*d1, *d2, *d3]),
        _ => None,
    };

    // apply mappings, if any
    let out_msg = if let Some([0x99, note, vel]) = out_msg {
        if let Some(new_note) = state
            .mappings
            .iter()
            .find_map(|(from, to)| if note == from { Some(to) } else { None })
        {
            Some(&[0x99, *new_note, *vel])
        } else {
            Some(&[0x99, *note, *vel])
        }
    } else {
        out_msg
    };

    if let Some(out_msg) = out_msg {
        if let Err(e) = state.out.send(out_msg) {
            error!("seding data to output: {e}");
        }
        trace!("processed note in {:#?}", start.elapsed());
    }
}
