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
    pub fn allocate_next<H: GarbageCollectingHeap>(
        &mut self,
        request: usize,
        allocator: &mut H,
    ) -> anyhow::Result<Pointer, HeapError> {
        match allocator.malloc(request, self) {
            Ok(p) => {
                self.allocations.push_back(p);
                Ok(p)
            }
            Err(e) => Err(e),
        }
    }

    pub fn deallocate_next(&mut self) -> Option<Pointer> {
        self.allocations.pop_front()
    }

    pub fn deallocate_any_that<F: Fn(Pointer)->bool>(&mut self, deallocate_condition: F) {
        self.allocations = self.allocations.iter().filter(|p| !deallocate_condition(**p)).copied().collect();
    }

    pub fn deallocate_next_even(&mut self) -> Option<Pointer> {
        if self.allocations.len() >= 2 {
            let popped = self.allocations.pop_front().unwrap();
            let result = self.allocations.pop_front();
            self.allocations.push_back(popped);
            result
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.allocations.len()
    }

    pub fn total_allocated(&self) -> usize {
        self.allocations.iter().map(|p| p.len()).sum()
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
