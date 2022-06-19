# Ezekiel

Ezekiel (consonant to easy-kyll, a simpler jekyll) is a "no bs" static website generator featuring

* [Tera](https://github.com/Keats/tera) templates.
* [Mmark](https://github.com/mmark-md/mmark) strict markdown processor (needs to be installed separately, e.g. `whalebrew install caphindsight/mmark-cli-dockerized`).

## Usage

```bash
$ whalebrew install caphindsight/ezekiel
$ ezekiel build
```

## Current state

Absolutely unusable dirty script with unreadable error messages.
Will generate your site according to:

1. All files / directories prefixed with `_` are private.
1. All non-private `html` files are rendered as Tera templates.
1. All non-private `md` files are rendered with Mmark and then Tera, the template name added in the yaml metadata section.
