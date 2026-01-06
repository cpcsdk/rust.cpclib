# Some recipes


## IDE Configuration for the bnd.build files

The description of the build language is provided in the file `https://raw.githubusercontent.com/cpcsdk/rust.cpclib/refs/heads/master/cpclib-bndbuild/schema.json`. Provide it to your editor to validate the files.

For examples the `yaml` settings of Visual Studio Code are:

```json
    "yaml.schemas": {
        "https://raw.githubusercontent.com/cpcsdk/rust.cpclib/refs/heads/master/cpclib-bndbuild/schema.json": ["bnd.build", "build.bnd", "bndbuild.yaml"]
    },
```

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

## I want to generate a graphical representation of the of the commands

`bndbuild --dot debug.png`
