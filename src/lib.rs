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
    pub fn matches<H: GarbageCollectingHeap>(&self, allocator: &H) -> bool {
        self.allocations
            .iter()
            .all(|p| allocator.is_allocated(p.block_num()))
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

    pub fn test_in_bounds<H: GarbageCollectingHeap>(&self, allocator: &mut H) {
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

pub struct CountdownTracer {
    counts: u64,
    count_ptr: Option<Pointer>,
}

impl Tracer for CountdownTracer {
    fn trace(&self, blocks_used: &mut [bool]) {
        self.count_ptr.map(|p| {
            blocks_used[p.block_num()] = true;
        });
    }
}

impl CountdownTracer {
    pub fn new<H: GarbageCollectingHeap>(start: u64, allocator: &mut H) -> Self {
        let mut result = Self {
            counts: start,
            count_ptr: None,
        };
        let literal_ptr = allocator.malloc(1, &mut result).unwrap();
        allocator.store(literal_ptr, start).unwrap();
        let stored = allocator.load(literal_ptr).unwrap();
        let count_ptr = allocator.malloc(1, &mut result).unwrap();
        allocator.store(count_ptr, stored).unwrap();
        result.count_ptr = Some(count_ptr);
        result
    }

    pub fn countdown_complete(&self) -> bool {
        self.counts == 0
    }

    pub fn report(&self) {
        println!("{} {:?}", self.counts, self.count_ptr.unwrap());
    }

    pub fn iterate<H: GarbageCollectingHeap>(&mut self, allocator: &mut H) {
        let p = allocator.malloc(1, self).unwrap();
        allocator.store(p, 0).unwrap();
        let count = allocator.load(self.count_ptr.unwrap()).unwrap();
        assert_eq!(count, self.counts);
        let zero = allocator.load(p).unwrap();
        assert_eq!(0, zero);
        let p = allocator.malloc(1, self).unwrap();
        allocator.store(p, 18446744073709551615).unwrap();
        let p = allocator.malloc(1, self).unwrap();
        allocator.store(p, 1).unwrap();

        println!("looking up {:?}", self.count_ptr.unwrap());
        let count = allocator.load(self.count_ptr.unwrap()).unwrap();
        assert_eq!(count, self.counts);
        let drop = allocator.load(p).unwrap();
        self.counts -= drop;
        let p = allocator.malloc(1, self).unwrap();
        allocator.store(p, self.counts).unwrap();
        self.count_ptr = Some(p);
    }
}
