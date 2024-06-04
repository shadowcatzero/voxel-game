use std::ops::{Deref, DerefMut, Add};

pub struct SwapBuffer<T> {
    read: Vec<T>,
    write: Vec<T>,
    modify: Vec<T>,
}

impl<T: Default + Copy + Clone + Add<Output = T>> SwapBuffer<T> {
    pub fn new(size: usize) -> Self {
        Self {
            read: vec![T::default(); size],
            write: vec![T::default(); size],
            modify: vec![T::default(); size],
        }
    }

    pub fn swap(&mut self) {
        std::mem::swap(&mut self.read, &mut self.write);
        for (m, r) in self.modify.iter_mut().zip(&mut self.read) {
            *r = *r + *m;
            *m = T::default();
        }
    }

    pub fn rwm(&mut self) -> (&mut Vec<T>, &mut Vec<T>, &mut Vec<T>) {
        (&mut self.read, &mut self.write, &mut self.modify)
    }
}

impl<T> Deref for SwapBuffer<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.read
    }
}

impl<T> DerefMut for SwapBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.read
    }
}

