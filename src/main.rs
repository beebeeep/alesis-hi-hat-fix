use std::io::stdin;

use anyhow::{Context, Result, anyhow};
use midir::{MidiInput, MidiInputPort, MidiOutput, MidiOutputConnection, os::unix::VirtualOutput};

struct OutState {
    out: MidiOutputConnection,
    hihat_pressed: bool,
}

fn main() -> Result<()> {
    let matches = clap::Command::new("alesis_hihat")
        .arg(clap::Arg::new("port").short('p'))
        .arg(clap::Arg::new("name").short('n'))
        .arg(
            clap::Arg::new("list")
                .short('l')
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let midi_in = MidiInput::new("alesis_hihat").context("initialising midi input")?;
    let midi_out = MidiOutput::new("alesis_hihat").context("initializing midi output")?;
    let out_port = match MidiOutput::create_virtual(midi_out, "alesis_hihat") {
        Err(e) => {
            return Err(anyhow!("creating virtual output port: {e}"));
        }
        Ok(v) => v,
    };
    let mut in_port: Option<MidiInputPort> = None;
    let state = OutState {
        out: out_port,
        hihat_pressed: false,
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
    }
    if let Some(name) = matches.get_one::<String>("name") {
        for p in midi_in.ports() {
            if midi_in.port_name(&p).unwrap().to_lowercase().contains(name) {
                in_port = Some(p);
                break;
            }
        }
    }
    if in_port.is_none() {
        return Err(anyhow!("no port found"));
    }

    let in_port = in_port.unwrap();
    println!("connecting to port {}", in_port.id());

    match midi_in.connect(&in_port, "midi test", handle_midi_data, state) {
        Err(e) => {
            return Err(anyhow!("connecting to midi input: {e}"));
        }
        Ok(_c) => {
            let mut i = String::new();
            println!("press Return to exit");
            let _ = stdin().read_line(&mut i);
        }
    };
    Ok(())
}

fn handle_midi_data(_ts: u64, message: &[u8], state: &mut OutState) -> () {
    // if message[0] != 0xf8 {
    //     // skip clock messages
    //     println!("{ts}: {message:?}");
    // }
    let out_msg = match message {
        [0xb9, 0x04, 0x00] => {
            // control change indicating the next note will be open hihat
            state.hihat_pressed = false;
            message
        }
        [0xb9, 0x04, 0x7f] => {
            // control change indicating the next note will be closed hihat
            state.hihat_pressed = true;
            message
        }
        [0x99, 0x2e, vel] => {
            if state.hihat_pressed {
                // replace note 46 (open hihat) to 42 (closed hihat)
                &[0x99, 0x2a, *vel]
            } else {
                &[0x99, 0x2e, *vel]
            }
        }
        v => v,
    };

    if let Err(e) = state.out.send(out_msg) {
        println!("seding data to output: {e}");
    }
}
