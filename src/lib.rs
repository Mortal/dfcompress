// Copyright 2018, Mathias Rav <m@git.strova.dk>
// SPDX-License-Identifier: LGPL-2.1+
extern crate flate2;

use flate2::read::{ZlibDecoder, ZlibEncoder};
use flate2::Compression;
use std::io::prelude::*;
use std::{fmt, io, result};

#[derive(Debug)]
pub enum ErrorKind {
    CompressionUnknown(u32),
    Io(io::Error),
    UnexpectedEof,
    VersionIsZero,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

pub type Result<T> = result::Result<T, Error>;

impl Into<Error> for ErrorKind {
    fn into(self) -> Error {
        Error { kind: self }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            ErrorKind::UnexpectedEof.into()
        } else {
            ErrorKind::Io(e).into()
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::CompressionUnknown(c) => write!(f, "Unknown compression {}", c),
            ErrorKind::Io(ref e) => write!(f, "{}", e),
            ErrorKind::UnexpectedEof => write!(f, "Unexpected end-of-file"),
            ErrorKind::VersionIsZero => write!(f, "Version is 0"),
        }
    }
}

fn read_u32(r: &mut io::Read) -> Result<u32> {
    let buf = &mut [0, 0, 0, 0];
    r.read_exact(buf)?;
    Ok(
        ((buf[3] as u32) << 24)
            + ((buf[2] as u32) << 16)
            + ((buf[1] as u32) << 8)
            + (buf[0] as u32),
    )
}

fn read_u32_or_eof(r: &mut io::Read) -> Result<Option<u32>> {
    match read_u32(r) {
        Ok(v) => Ok(Some(v)),
        Err(e) => match e.kind {
            ErrorKind::UnexpectedEof => Ok(None),
            _ => Err(e),
        },
    }
}

fn write_u32(handle: &mut io::Write, value: u32) -> Result<()> {
    let buf = &[
        value as u8,
        (value >> 8) as u8,
        (value >> 16) as u8,
        (value >> 24) as u8,
    ];
    handle.write_all(buf)?;
    Ok(())
}

fn read_header(stdin: &mut io::Read) -> Result<(u32, u32)> {
    let version = read_u32(stdin)?;
    if version == 0 {
        return Err(ErrorKind::VersionIsZero.into());
    }
    let compression = read_u32(stdin)?;
    if compression > 1 {
        return Err(ErrorKind::CompressionUnknown(compression).into());
    }
    Ok((version, compression))
}

pub fn dfuncompress(stdin: &mut io::Read, stdout: &mut io::Write) -> Result<()> {
    let (version, compression) = read_header(stdin)?;
    write_u32(stdout, version)?;
    write_u32(stdout, 0)?;
    if compression == 0 {
        io::copy(stdin, stdout)?;
    } else {
        let mut buf = Vec::new();
        loop {
            let n = match read_u32_or_eof(stdin)? {
                Some(v) => v as u64,
                None => break,
            };
            buf.clear();
            ZlibDecoder::new(stdin.take(n)).read_to_end(&mut buf)?;
            stdout.write_all(&buf)?;
        }
    }
    Ok(())
}

pub fn dfcompress(stdin: &mut io::Read, stdout: &mut io::Write) -> Result<()> {
    let (version, compression) = read_header(stdin)?;
    write_u32(stdout, version)?;
    write_u32(stdout, 1)?;
    if compression == 1 {
        io::copy(stdin, stdout)?;
    } else {
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let mut encoder = ZlibEncoder::new(stdin.take(20000), Compression::default());
            encoder.read_to_end(&mut buf)?;
            if encoder.total_in() == 0 {
                break;
            }
            write_u32(stdout, buf.len() as u32)?;
            stdout.write_all(&buf)?;
        }
    }
    Ok(())
}

#[test]
fn u32_tests() {
    fn read_help(d: Vec<u8>) -> u32 {
        read_u32(&mut io::Cursor::new(&d)).unwrap()
    }

    fn write_help(d: u32) -> Vec<u8> {
        let mut b = io::Cursor::new(vec![0, 0, 0, 0]);
        write_u32(&mut b, d).unwrap();
        b.into_inner()
    }

    assert_eq!(read_help(vec![42, 0, 0, 0]), 42);
    assert_eq!(
        read_help(vec![1, 2, 3, 4]),
        (4 << 24) + (3 << 16) + (2 << 8) + 1
    );
    assert_eq!(read_help(write_help(11111111)), 11111111);
}

#[test]
fn compress_test() {
    let mut buf = Vec::new();
    buf.resize(30000, b'a');
    let buf = {
        let mut cursor = io::Cursor::new(buf);
        write_u32(&mut cursor, 1234).unwrap(); // version
        write_u32(&mut cursor, 0).unwrap(); // compression
        cursor.into_inner()
    };
    let mut buf2 = Vec::new();
    dfcompress(&mut io::Cursor::new(&buf), &mut io::Cursor::new(&mut buf2)).unwrap();
    let mut buf3 = Vec::new();
    dfuncompress(&mut io::Cursor::new(&buf2), &mut io::Cursor::new(&mut buf3)).unwrap();
    assert_eq!(&buf, &buf3);
}
