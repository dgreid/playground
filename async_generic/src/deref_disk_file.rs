// Test deref to DiskFile

use std::ops::{Deref, DerefMut};

trait A {
    fn a(&self);
}

trait B {
    fn b(&self);
}

trait DiskFile: A + B {}
impl<T> DiskFile for T where T: A + B {}

struct File {
    val: u32,
}
impl A for File {
    fn a(&self) {}
}
impl B for File {
    fn b(&self) {}
}

trait AsyncDisk {
    fn read_it(&self);
    fn inner(&self) -> &dyn DiskFile;
}

struct SingleFileDisk {
    file: File,
}

impl AsyncDisk for SingleFileDisk {
    fn read_it(&self) {}
    fn inner(&self) -> &dyn DiskFile {
        &self.file
    }
}

impl Deref for SingleFileDisk {
    type Target = File;
    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl DerefMut for SingleFileDisk {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

struct OldBlock {
    disk: Box<dyn DiskFile>,
}

impl OldBlock {
    fn do_a(&self) {
        self.disk.a()
    }
}

struct Block {
    disk: Box<dyn AsyncDisk>,
}

impl Block {
    fn do_a(&self) {
        self.disk.inner().a()
    }
}

pub fn run_test() {
    let file = File { val: 0 };
    let block = OldBlock {
        disk: Box::new(file),
    };

    let file = File { val: 0 };
    let disk = SingleFileDisk { file };
    let block = Block {
        disk: Box::new(disk),
    };
}
