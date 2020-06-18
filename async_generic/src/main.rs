use std::future::Future;

mod deref_disk_file;

trait VirtioDevice {
    fn run(&self);
}

struct Block<F, Fut>
where
    F: Fn(u32) -> Fut,
    Fut: Future<Output = u32>,
{
    process_op: F,
}

impl<F, Fut> Block<F, Fut>
where
    F: Fn(u32) -> Fut,
    Fut: Future<Output = u32>,
{
    fn new(f: F) -> Self {
        Block { process_op: f }
    }

    async fn do_op(&self) -> u32 {
        (self.process_op)(3).await
    }
}

impl<F, Fut> VirtioDevice for Block<F, Fut>
where
    F: Fn(u32) -> Fut,
    Fut: Future<Output = u32>,
{
    fn run(&self) {
        let fut = self.do_op();
        futures::pin_mut!(fut);
        cros_async::run_one(fut).unwrap();
    }
}

fn test() {
    async fn op_proc_file(b: u32) -> u32 {
        // This would process read/write etc for files.
        b + 32
    }

    async fn op_proc_qcow(disk_op: u32) -> u32 {
        async fn get_index(offset: u32) -> u32 {
            offset + 5
        }
        async fn read_from_index(index: u32) -> u32 {
            index + 2
        }

        let index = get_index(disk_op).await;
        read_from_index(index).await
    }

    let mut blocks: Vec<Box<dyn VirtioDevice>> = Vec::new();
    blocks.push(Box::new(Block::new(op_proc_file)));
    blocks.push(Box::new(Block::new(op_proc_qcow)));
    for block in &blocks {
        block.run();
    }
}

fn main() {
    test();
}
