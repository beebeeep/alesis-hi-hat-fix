# alesis-hi-hat-fix

Alesis e-drums MIDI output has one annoying quirk - hihat is sending note 46 (open hi-hat) regardless of hi-hat pedal.
This small tool fixes that by properly handing Control Change sequences that kit sends alongside hi-hat notes and playing note 42 if hi-hat is closed.

# usage
Run program to listen specific MIDI input:
```
hi-hat-fix -l # list all midi inputs
hi-hat-fix -p 16:0 # to read from port 16:0
``` 
It will create new virtual port called `alesis_hihat` - that's where you shall connect in your DAW/sampler/whater else drum program you use.


