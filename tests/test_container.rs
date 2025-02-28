mod utils;

use crate::utils::{gzip_data, xz_data, zstd_data};
use anyreader::read_recursive;
use std::path::{Path, PathBuf};
use tracing_test::traced_test;

const DATA: &[u8] = b"hello world";

fn process(data: &[u8]) -> Vec<(PathBuf, Vec<u8>)> {
    let mut result = Vec::new();
    read_recursive(Path::new("root"), data, &mut |item| {
        let mut buf = Vec::new();
        item.reader.read_to_end(&mut buf).unwrap();
        result.push((item.path, buf));
    });
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
