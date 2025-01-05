use std::collections::VecDeque;

use gc_headers::{GarbageCollectingHeap, HeapError, Pointer, Tracer};

#[derive(Default, Debug)]
pub struct TestTracer {
    allocations: VecDeque<Pointer>,
}

impl Tracer for TestTracer {
    fn trace(&self, blocks_used: &mut [bool]) {
        for p in self.allocations.iter() {
            blocks_used[p.block_num()] = true;
        }
    }
}

impl TestTracer {
    pub fn matches<H: GarbageCollectingHeap>(
        &self,
        allocator: &H,
    ) -> bool {
        self.allocations.iter().all(|p| allocator.is_allocated(p.block_num()))
    }

    pub fn allocate_next<H: GarbageCollectingHeap>(
        &mut self,
        request: usize,
        allocator: &mut H,
    ) -> anyhow::Result<(), HeapError> {
        match allocator.malloc(request, self) {
            Ok(p) => {
                self.allocations.push_back(p);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn deallocate_next_even(&mut self) {
        if self.allocations.len() >= 2 {
            let popped = self.allocations.pop_front().unwrap();
            self.allocations.pop_front().unwrap();
            self.allocations.push_back(popped);
        }
    }

    pub fn len(&self) -> usize {
        self.allocations.len()
    }

    pub fn total_allocated(&self) -> usize {
        self.allocations.iter().map(|p| p.len()).sum()
    }

    pub fn test_in_bounds<H: GarbageCollectingHeap>(
        &self,
        allocator: &mut H,
    ) {
        let mut value = 0;
        for p in self.allocations.iter() {
            let len = p.len();
            let mut p = Some(*p);
            for _ in 0..len {
                let pt = p.unwrap();
                allocator.store(pt, value).unwrap();
                assert_eq!(value, allocator.load(pt).unwrap());
                value += 1;
                p = pt.next();
            }
        }

        value = 0;
        for p in self.allocations.iter() {
            let len = p.len();
            let mut p = Some(*p);
            for _ in 0..len {
                let pt = p.unwrap();
                assert_eq!(value, allocator.load(pt).unwrap());
                value += 1;
                p = pt.next();
            }
        }
    }
}