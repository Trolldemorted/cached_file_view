extern crate cached_file_view;

use std::fs::File;
use std::path::Path;
use cached_file_view::FileView;
use cached_file_view::FileViewError;


#[test]
fn basic() {
    let f = File::open(Path::new("tests/test.txt")).expect("file not found");
    let view1 = FileView::new(f).unwrap();
    let view2 = view1.clone();
    let full_mapping = view1.read(0..12).unwrap();
    let small_mapping = view2.read(3..12).unwrap();

    let full = full_mapping.to_vec();
    let small = small_mapping.to_vec();
    assert!(full == [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33]);
    assert!(small == [108, 111, 32, 119, 111, 114, 108, 100, 33]);
}

#[test]
fn test_read1() {
    let f = File::open(Path::new("tests/test.txt")).expect("file not found");
    let view = FileView::new(f).unwrap();
    assert!(view.read(0..12).unwrap().to_vec() == "hello world!".as_bytes());
}

#[test]
fn test_read2() {
    let f = File::open(Path::new("tests/test.txt")).expect("file not found");
    let view = FileView::new(f).unwrap();
    assert!(view.read(11..12).unwrap().to_vec() == "!".as_bytes());
}

#[test]
fn test_oob_end1() {
    let f = File::open(Path::new("tests/test.txt")).expect("file not found");
    let view = FileView::new(f).unwrap();

    if let Err(err) = view.read(0..13) {
        assert!(err == FileViewError::OutOfBoundsError);
    } else {
        panic!("oob read did not fail")
    }
}

#[test]
fn test_oob_end2() {
    let f = File::open(Path::new("tests/test.txt")).expect("file not found");
    let view = FileView::new(f).unwrap();

    if let Err(err) = view.read(&(10..13)) {
        assert!(err == FileViewError::OutOfBoundsError);
    } else {
        panic!("oob read did not fail")
    }
}

#[test]
fn test_huge() {
    let f = File::open(Path::new("../twa_unpack/twa_rigidmodels.pack")).expect("file not found");
    let view = FileView::new(f).unwrap();
    {
        let _ = view.read(0x0..0x4);
        let _ = view.read(0x34..0x508DF);
    }
    
    println!("#######################");
    let v = view.read(0x508DF..0xE50FF).unwrap().to_vec();
    println!("{:x?}", &v[0..4]);
    assert!(&v[0..4] == "RMV2".as_bytes());
}