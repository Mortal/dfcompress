// Copyright 2018, Mathias Rav <m@git.strova.dk>
// SPDX-License-Identifier: LGPL-2.1+
extern crate dfcompress;
use std::io;
use std::process;

fn main() {
    process::exit({
        let raw_stdin = io::stdin();
        let stdin = raw_stdin.lock();
        let raw_stdout = io::stdout();
        let stdout = raw_stdout.lock();
        match dfcompress::dfcompress(stdin, stdout) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{}", e);
                1
            }
        }
    });
}
