# Panrelease

## Quick start

Install using npm:

```shell
npm i --save-dev panrelease
```

Or yarn:

```shell
yarn add -D panrelease
```

Write a configuration file:

```toml
[vcs]
software = "Git"

[modules.root]
path = "."
packageManager = "Npm"
```

Add custom script in your package.json

```json
{
  "scripts": {
    "rel": "panrelease release"
  }
}
```