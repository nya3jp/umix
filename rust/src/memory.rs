use std::ops::{Index, IndexMut};

#[derive(Clone, Default, Debug)]
pub struct Arrays {
    arrays: Vec<Option<Vec<u32>>>,
    ptrs: Vec<*mut u32>,
    vacants: Vec<usize>,
}

impl Arrays {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, mut array: Vec<u32>) -> usize {
        match self.vacants.pop() {
            Some(id) => {
                self.ptrs[id] = array.as_mut_ptr();
                self.arrays[id] = Some(array);
                assert_eq!(
                    self.arrays[id].as_mut().unwrap().as_mut_ptr(),
                    self.ptrs[id]
                );
                id
            }
            None => {
                let id = self.arrays.len();
                self.ptrs.push(array.as_mut_ptr());
                self.arrays.push(Some(array));
                assert_eq!(
                    self.arrays[id].as_mut().unwrap().as_mut_ptr(),
                    self.ptrs[id]
                );
                id
            }
        }
    }

    pub fn remove(&mut self, id: usize) {
        assert!(self.arrays[id].is_some());
        self.arrays[id] = None;
        self.ptrs[id] = std::ptr::null_mut();
        self.vacants.push(id);
    }

    pub fn dup0(&mut self, id: usize) {
        if id == 0 {
            return;
        }
        self.arrays[0] = self.arrays[id].clone();
        self.ptrs[0] = self.arrays[0].as_mut().unwrap().as_mut_ptr();
    }

    pub fn as_mut_ptr(&mut self) -> *mut *mut u32 {
        self.ptrs.as_mut_ptr()
    }
}

impl Index<usize> for Arrays {
    type Output = [u32];

    fn index(&self, id: usize) -> &Self::Output {
        self.arrays.get(id).unwrap().as_ref().unwrap()
    }
}

impl IndexMut<usize> for Arrays {
    fn index_mut(&mut self, id: usize) -> &mut Self::Output {
        self.arrays.get_mut(id).unwrap().as_mut().unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Memory {
    pub regs: [u32; 8],
    pub arrays: Arrays,
}

impl Memory {
    pub fn new(program: Vec<u32>) -> Self {
        let mut arrays = Arrays::new();
        let id = arrays.insert(program);
        assert_eq!(id, 0);
        Self {
            regs: [0; 8],
            arrays,
        }
    }
}
