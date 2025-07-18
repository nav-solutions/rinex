CONTRIBUTING
============

Hello and welcome on board :wave:

Use the github portal to submit PR, all contributions are welcomed.  
Don't forget to run a quick `cargo fmt` prior any submissions, so the CI/CD does not fail
on coding style "issues".

This crate and ecosystem is part of the [RTK-rs framework and community](https://github.com/nav-solutions).  
You can contact us on [Discord](https://discord.com/invite/VwuKPcw6)

Crate architecture
==================

The RINEX library is architectured per RINEX formats. We support

- Observation RINEX
- Navigation RINEX
- IONEX
- Clock RINEX
- ANTEX
- DORIS 
- and Meteo RINEX

For each supported RINEX types, you will find the related to section in a dedicated folder, 
for example `src/navigation` for Navigation RINEX.

For each record type, we define two custom types, `K` and `V` to describe the file content and be 
indexed in a Map (either Hash or BTree). This principle has proved sufficient and very powerful.
The crate is undergoing a simplification phase where the record types are being simplified.
In otherwords, a simple `Map<K, V>` is then enough to represent the file content: that was not the case
in previous versions. This is true for most types but not all of them as of today!

For that reason, this parser requires the `std` lib and we don't have plans to remove that dependency.

## Key external structures

`SV` and `Constellation` support is provided by our general purposes GNSS library.  
`Epoch`, `Duration` and `Timescales` are defined within `Hifitime`.

## Key internal structures and elements

- `src/observable` desfines RINEX Observable, as used in many RINEX formats.
- `src/carrier` defines carrier signals

## Notes on CRINEX (RINEX compression)

Support for the CRINEX algorithm and compression technique
is provided in the `src/hatanaka` module, named after Y. Hatanaka who invented this compression algorithm, dedicated to Observation RINEX.

We have a very complete & powerful infrastructure that support the algorithm both ways and have demonstrated 
bit by bit reciprocity. The algorithm is tested in many ways in our CI and benchmarked on github.com.
This makes our library the most complete when it comes to supporting this compression technique as of today.

## Build Script

The crate comes with a build script that defines the Navigation messages to be supported (written as dictionary).
