# Some recipes


## Launch of embedded commands

These commands are not included in `bndbuild` source code and are downloaded,
eventually compiled, installed in a cache folder of `bndbuild`.
`bndbuild` can serve as a proxy to use them without manually installing them.



### ACE

`bndbuild --direct -- ace [ace arguments]`


### rasm

`bndbuild --direct -- rasm [rasm arguments]`


## Update of commands

Once embedded commands have been downloaded and installed, there is no need to reinstall them.
Version number is hardcoded in `bndbuild` source code.
However, some of them have not a clear version management form their authors. As a sideback,
a new download can imply a new version. We provide then a dedicated command that.
It can be usefull also in case of a former failed installation.

### Update rasm

`bndbuild --update rasm`



### Update ACE

`bndbuild --update ace`