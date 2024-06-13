use std::{ops::Deref, sync::mpsc::{channel, Receiver, Sender}};

pub type EntityID = usize;

pub struct Component<T> {
    id: EntityID,
    val: T,
}

impl<T> Deref for Component<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

pub struct ComponentVec<T> {
    vec: Vec<Component<T>>,
    changes: Vec<EntityID>,
}

impl<'a, T> IntoIterator for &'a ComponentVec<T> {
    type Item = &'a Component<T>;
    type IntoIter = std::slice::Iter<'a, Component<T>>;
    fn into_iter(self) -> Self::IntoIter {
        (&self.vec).into_iter()
    }
}

pub struct ComponentMut<'a, T> {
    cmp: &'a Component<T>,
    send: Sender<EntityID>,
}

pub struct ComponentMutIter<'a, T> {
    iter: std::slice::IterMut<'a, Component<T>>,
    recv: Receiver<EntityID>,
    send: Sender<EntityID>,
    i: usize,
}

impl<'a, T> Iterator for ComponentMutIter<'a, T> {
    type Item = ComponentMut<'a, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|c| ComponentMut {
            cmp: c,
            send: self.send.clone(),
        })
    }
}

impl<'a, T> IntoIterator for &'a mut ComponentVec<T> {
    type Item = ComponentMut<'a, T>;
    type IntoIter = ComponentMutIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        let (send, recv) = channel();
        ComponentMutIter {
            iter: (&mut self.vec).into_iter(),
            recv,
            send,
            i: 0,
        }
    }
}
