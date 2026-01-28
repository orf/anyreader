mod utils;

use crate::utils::{gzip_data, xz_data, zstd_data};
use anyreader::{iterate_archive, recursive_read};
use std::path::{Path, PathBuf};
use tracing_test::traced_test;

const DATA: &[u8] = b"hello world";

fn process(data: &[u8]) -> Vec<(PathBuf, Vec<u8>)> {
    let mut result = Vec::new();
    recursive_read(Path::new("root"), data, &mut |item| {
        let mut buf = Vec::new();
        item.reader.read_to_end(&mut buf).unwrap();
        result.push((item.path, buf));
        Ok(())
    })
    .unwrap();
    result
}

#[traced_test]
#[test]
fn test_tar() {
    let archive: Vec<u8> = utils::tar_archive([
        ("file-1", xz_data(zstd_data(gzip_data(DATA)))),
        ("inner", utils::tar_archive([("file-2", DATA.to_vec())])),
    ]);
    let x = process(archive.as_slice());
    assert_eq!(
        x,
        vec![
            ("root/file-1".into(), DATA.to_vec()),
            ("root/inner/file-2".into(), DATA.to_vec()),
        ]
    );
}

#[traced_test]
#[test]
fn test_zip() {
    let archive: Vec<u8> = utils::zip_archive([
        ("file-1", xz_data(zstd_data(gzip_data(DATA)))),
        ("inner", utils::zip_archive([("file-2", DATA.to_vec())])),
    ]);
    let x = process(archive.as_slice());
    assert_eq!(
        x,
        vec![
            ("root/file-1".into(), DATA.to_vec()),
            ("root/inner/file-2".into(), DATA.to_vec()),
        ]
    );
}

#[traced_test]
#[test]
fn test_mixed() {
    let archive: Vec<u8> = utils::zip_archive([
        ("file-1", xz_data(zstd_data(gzip_data(DATA)))),
        ("inner", utils::tar_archive([("file-2", DATA.to_vec())])),
    ]);
    let x = process(archive.as_slice());
    assert_eq!(
        x,
        vec![
            ("root/file-1".into(), DATA.to_vec()),
            ("root/inner/file-2".into(), DATA.to_vec()),
        ]
    );
}

#[traced_test]
#[test]
fn test_text() {
    let x = process(DATA);
    assert_eq!(x, vec![("root".into(), DATA.to_vec())]);
}

// Tests for iterate_archive

#[traced_test]
#[test]
fn test_iterate_archive_tar_gz() {
    let archive = gzip_data(utils::tar_archive([
        ("file-1.txt", DATA.to_vec()),
        ("file-2.gz", gzip_data(DATA)), // Should NOT be auto-decompressed
    ]));

    let mut entries = Vec::new();
    iterate_archive(archive.as_slice(), |item| {
        let mut buf = Vec::new();
        item.reader.read_to_end(&mut buf)?;
        entries.push((item.path.clone(), buf));
        Ok(())
    })
    .unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].0, PathBuf::from("file-1.txt"));
    assert_eq!(entries[0].1, DATA); // Plain file
    assert_eq!(entries[1].0, PathBuf::from("file-2.gz"));
    assert_eq!(entries[1].1, gzip_data(DATA)); // Still compressed
}

#[traced_test]
#[test]
fn test_iterate_archive_nested_no_recursion() {
    // Create a tar.gz containing a nested tar archive
    let inner_tar = utils::tar_archive([("inner-file.txt", DATA.to_vec())]);
    let archive = gzip_data(utils::tar_archive([
        ("outer-file.txt", DATA.to_vec()),
        ("nested.tar", inner_tar.clone()),
    ]));

    let mut entries = Vec::new();
    iterate_archive(archive.as_slice(), |item| {
        let mut buf = Vec::new();
        item.reader.read_to_end(&mut buf)?;
        entries.push((item.path.clone(), buf));
        Ok(())
    })
    .unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].0, PathBuf::from("outer-file.txt"));
    assert_eq!(entries[0].1, DATA);
    // The nested tar should be returned as raw bytes, not recursed into
    assert_eq!(entries[1].0, PathBuf::from("nested.tar"));
    assert_eq!(entries[1].1, inner_tar);
}

#[traced_test]
#[test]
fn test_iterate_archive_compressed_entry() {
    // Verify that compressed entries are NOT auto-decompressed
    let compressed_data = zstd_data(xz_data(gzip_data(DATA)));
    let archive = utils::tar_archive([("compressed.zst.xz.gz", compressed_data.clone())]);

    let mut entries = Vec::new();
    iterate_archive(archive.as_slice(), |item| {
        let mut buf = Vec::new();
        item.reader.read_to_end(&mut buf)?;
        entries.push((item.path.clone(), buf));
        Ok(())
    })
    .unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, PathBuf::from("compressed.zst.xz.gz"));
    // Should still be the original compressed bytes
    assert_eq!(entries[0].1, compressed_data);
}

#[traced_test]
#[test]
fn test_iterate_archive_not_an_archive() {
    // Raw data should error
    let result = iterate_archive(DATA, |_| Ok(()));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);

    // Compressed non-archive should also error
    let result = iterate_archive(gzip_data(DATA).as_slice(), |_| Ok(()));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);

    // Multiple compression layers with no archive should error
    let result = iterate_archive(zstd_data(gzip_data(DATA)).as_slice(), |_| Ok(()));
    assert!(result.is_err());
}

#[traced_test]
#[test]
fn test_iterate_archive_zip() {
    let archive = utils::zip_archive([
        ("file-1.txt", DATA.to_vec()),
        ("file-2.gz", gzip_data(DATA)),
    ]);

    let mut entries = Vec::new();
    iterate_archive(archive.as_slice(), |item| {
        let mut buf = Vec::new();
        item.reader.read_to_end(&mut buf)?;
        entries.push((item.path.clone(), buf));
        Ok(())
    })
    .unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].0, PathBuf::from("file-1.txt"));
    assert_eq!(entries[0].1, DATA);
    assert_eq!(entries[1].0, PathBuf::from("file-2.gz"));
    assert_eq!(entries[1].1, gzip_data(DATA)); // Still compressed
}
