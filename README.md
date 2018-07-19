Compression programs for Dwarf Fortress
=======================================

This project provides two programs for converting between "compressed saves"
and "uncompressed saves" in Dwarf Fortress.

The programs accept binary data on standard input
and produce binary data on standard out.

Usage
-----

1. [Install Rust and Cargo](https://www.rust-lang.org/en-US/install.html)
2. `cargo build --release`
3. `target/release/dfcompress < path/to/world.dat > world-compressed.dat`
4. `target/release/dfuncompress < world-compressed.dat > world-uncompressed.dat`

Usage with Git
--------------

Store uncompressed saves in Git, compressed saves on disk:

1. `cd ~/.dwarffortress`
2. `git init`
3. `git config filter.dfcompress.clean /path/to/target/release/dfcompress`
4. `git config filter.dfcompress.smudge /path/to/target/release/dfuncompress`
5. `echo '*.dat filter=dfcompress' >> .gitattributes`
6. `echo '*.sav filter=dfcompress' >> .gitattributes`
7. `git add .gitattributes data/save/region*`
8. `git commit -m Savescumming`

Format
------

Dwarf Fortress v0.44.12 compresses data files as follows:
The data file consists of a 32-bit little endian version,
followed by a 32-bit little endian compression flag.
The file is "uncompressed" if the flag is 0, and "compressed" if the flag is 1.

Uncompressed files consist of data segments of 20000 bytes after the header;
the last data segment may be shorter.

Compression works on each data segment independently:
Each segment is zlib-compressed and replaced with its compressed size
in 32-bit little endian followed by the zlib data.

License
-------

Copyright 2018, Mathias Rav <m@git.strova.dk>.
Licensed under [LGPL-2.1+](https://spdx.org/licenses/LGPL-2.1.html).
