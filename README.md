# alesis-hi-hat-fix

Alesis e-drums MIDI output has one annoying quirk - hihat is sending note 46 (open hi-hat in General MIDI) regardless of hi-hat pedal status.
This small tool fixes that by properly handing Control Change sequences that kit sends alongside hi-hat notes and playing note 42 instead if hi-hat is closed.

# Usage
Run program to listen specific MIDI input or without any arguments to let it try automatically detect connected Alesis kit:
```
hi-hat-fix -l # list all midi inputs
hi-hat-fix -p 16:0 # to read from port 16:0
``` 
It will create new virtual port (by default called `alesis_hihat`) - that's where you shall connect in your DAW/sampler/whatever else drum program you use.

# Bonus
Pass `-d` option and turn your hi-hat pedal into second kick pedal to crank some blastbits! Hi-hat will always be open, because who needs closed hi-hat in that case, right?


