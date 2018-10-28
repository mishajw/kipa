# To do

## Features
- [ ] Add prompts for copying user's private key. These should clearly state
  where the key will be copied, that it will still be fully encrypted, and ask
  for permission.
- [ ] Allow different methods for passing private key passphrases, e.g. via
  `STDIN`. Currently we only support via file.
- [ ] Add UDP support for faster search times.
- [ ] Add configurable output for search commands, e.g. only print IP, only
  print port, print both.

## Improvements
- [ ] Add levels of verbose modes: warning logs, info logs, all logs
- [ ] Add configuration file.
