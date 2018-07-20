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

fn read_u32<R: io::Read>(r: &mut R) -> Result<u32> {
    let buf = &mut [0, 0, 0, 0];
    r.read_exact(buf)?;
    Ok(
        ((buf[3] as u32) << 24) + ((buf[2] as u32) << 16) + ((buf[1] as u32) << 8)
            + (buf[0] as u32),
    )
}

fn read_u32_or_eof<R: io::Read>(r: &mut R) -> Result<Option<u32>> {
    match read_u32(r) {
        Ok(v) => Ok(Some(v)),
        Err(e) => match e.kind {
            ErrorKind::UnexpectedEof => Ok(None),
            _ => Err(e),
        },
    }
}

fn write_u32<W: io::Write>(handle: &mut W, value: u32) -> Result<()> {
    let buf = &[
        value as u8,
        (value >> 8) as u8,
        (value >> 16) as u8,
        (value >> 24) as u8,
    ];
    handle.write_all(buf)?;
    Ok(())
}

fn read_header<R: io::Read>(stdin: &mut R) -> Result<(u32, u32)> {
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

pub fn dfuncompress<R: io::Read, W: io::Write>(mut stdin: R, mut stdout: W) -> Result<()> {
    let (version, compression) = read_header(&mut stdin)?;
    write_u32(&mut stdout, version)?;
    write_u32(&mut stdout, 0)?;
    if compression == 0 {
        io::copy(&mut stdin, &mut stdout)?;
    } else {
        let mut buf = Vec::new();
        loop {
            let n = match read_u32_or_eof(&mut stdin)? {
                Some(v) => v as u64,
                None => break,
            };
            buf.clear();
            ZlibDecoder::new((&mut stdin).take(n)).read_to_end(&mut buf)?;
            stdout.write_all(&buf)?;
        }
    }
    Ok(())
}

pub fn dfcompress<R: io::Read, W: io::Write>(mut stdin: R, mut stdout: W) -> Result<()> {
    let (version, compression) = read_header(&mut stdin)?;
    write_u32(&mut stdout, version)?;
    write_u32(&mut stdout, 1)?;
    if compression == 1 {
        io::copy(&mut stdin, &mut stdout)?;
    } else {
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let mut encoder = ZlibEncoder::new((&mut stdin).take(20000), Compression::default());
            encoder.read_to_end(&mut buf)?;
            if encoder.total_in() == 0 {
                break;
            }
            write_u32(&mut stdout, buf.len() as u32)?;
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
    let mut buf = io::Cursor::new(Vec::new());
    buf.get_mut().resize(30000, b'a');
    write_u32(&mut buf, 1234).unwrap(); // version
    write_u32(&mut buf, 0).unwrap(); // compression
    buf.seek(io::SeekFrom::Start(0)).unwrap();
    let mut buf2 = io::Cursor::new(Vec::new());
    dfcompress(&mut buf, &mut buf2).unwrap();
    buf2.seek(io::SeekFrom::Start(0)).unwrap();
    let mut buf3 = io::Cursor::new(Vec::new());
    dfuncompress(&mut buf2, &mut buf3).unwrap();
    assert_eq!(buf.get_ref(), buf3.get_ref());
}
