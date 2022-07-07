/***
 * Implement a doubley linked list in rust
 ***/
use std::cell::RefCell;
use std::rc::{Rc, Weak};
#[derive(Debug, Default)]
struct Node<T: std::fmt::Debug + Default + Clone> {
    key: T,
    next: Option<Rc<RefCell<Node<T>>>>,
    prev: Option<Weak<RefCell<Node<T>>>>,
}

impl<T: std::fmt::Debug + Default + Clone> Node<T> {
    pub fn new(key: T) -> Self {
        Self {
            key,
            next: None,
            prev: None,
        }
    }
}

impl<T: std::fmt::Debug + Default + Clone> From<Node<T>> for Option<Rc<RefCell<Node<T>>>> {
    fn from(node: Node<T>) -> Self {
        Some(Rc::new(RefCell::new(node)))
    }
}
#[derive(Debug)]
pub struct List<T: std::fmt::Debug + Default + Clone> {
    head: Option<Rc<RefCell<Node<T>>>>,
    tail: Option<Rc<RefCell<Node<T>>>>,
}

impl<T: std::fmt::Debug + Default + Clone> List<T> {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }
    //Push to the front of the list
    pub fn push_front(&mut self, key: T) {
        let node = Node::new(key).into();
        match self.head {
            None => {
                self.head = node;
                self.tail = self.head.as_ref().map(|node| Rc::clone(node));
            }
            Some(ref mut head) => {
                head.borrow_mut().prev = node.as_ref().map(|node| Rc::downgrade(&Rc::clone(node)));
                self.head = node.map(|node| {
                    node.borrow_mut().next = Some(Rc::clone(head));
                    node
                });
            }
        }
    }
    //Push to the back of the list
    pub fn push_back(&mut self, key: T) {
        let mut node = Node::new(key).into();
        match self.tail {
            None => {
                self.head = node;
                self.tail = self.head.as_ref().map(|node| Rc::clone(node));
            }
            Some(ref mut tail) => {
                self.tail = node.take().map(|node| {
                    node.borrow_mut().prev = Some(Rc::downgrade(&Rc::clone(tail)));
                    tail.borrow_mut().next = Some(Rc::clone(&node));
                    node
                });
            }
        }
    }

    //Pop out from the back of the list
    pub fn pop_back(&mut self) -> Option<T> {
        match self.tail.take() {
            None => None,
            Some(ref mut tail) => {
                self.tail = tail.borrow().prev.as_ref().and_then(|prev| {
                    let prev = prev.upgrade().map(|prev| {
                        prev.borrow_mut().next = None;
                        prev
                    });
                    prev
                });
                if self.tail.is_none() {
                    self.head.take();
                }
                //Use of default
                Some(tail.take().key)
            }
        }
    }

    //Pop out from the front of the list
    pub fn pop_front(&mut self) -> Option<T> {
        match self.head.take() {
            None => None,
            Some(ref mut head) => {
                self.head = head.borrow_mut().next.take().map(|next| {
                    next.borrow_mut().prev.take();
                    next
                });
                if self.head.is_none() {
                    self.tail.take();
                }
                //Use of default
                Some(head.take().key)
            }
        }
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            head: self.head.as_ref().map(Rc::clone),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut { list: self }
    }
}

pub struct Iter<T: std::fmt::Debug + Default + Clone> {
    head: Option<Rc<RefCell<Node<T>>>>,
}

//Itearor that returns Option<T>
//Values are cloned
//Underlying list remain intact
impl<T: std::fmt::Debug + Default + Clone> Iterator for Iter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.head {
            None => None,
            Some(_) => {
                match self.head.as_ref().map(|head| {
                    let head_node = head.borrow();
                    (
                        //Use of clone
                        head_node.key.clone(),
                        head_node.next.as_ref().map(Rc::clone),
                    )
                }) {
                    None => None,
                    Some(key_and_next_head) => {
                        self.head = key_and_next_head.1;
                        Some(key_and_next_head.0)
                    }
                }
            }
        }
    }
}

pub struct IterMut<'a, T: std::fmt::Debug + Default + Clone> {
    list: &'a mut List<T>,
}

//Iterator that consumes the list elements from the front
impl<'a, T: std::fmt::Debug + Default + Clone> Iterator for IterMut<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.list.pop_front()
    }
}

struct TreeIterMut<T: std::fmt::Debug + Default + Clone> {
    next: Option<Rc<RefCell<Node<T>>>>,
}

impl<T: std::fmt::Debug + Default + Clone> Iterator for TreeIterMut<T> {
    type Item = Rc<RefCell<Node<T>>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next {
            Some(_) => {
                match self
                    .next
                    .as_ref()
                    .map(|next| (Rc::clone(next), next.borrow().next.as_ref().map(Rc::clone)))
                {
                    None => None,
                    Some(current_and_next) => {
                        self.next = current_and_next.1;
                        Some(current_and_next.0)
                    }
                }
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_push_and_pop_front() {
        let mut list = List::new();
        list.push_front(1);
        list.push_front(2);
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), None);
    }
    #[test]
    fn test_push_and_pop_back() {
        let mut list = List::new();
        list.push_back(1);
        list.push_back(2);
        assert_eq!(list.pop_back(), Some(2));
        assert_eq!(list.pop_back(), Some(1));
        assert_eq!(list.pop_back(), None);
        assert_eq!(list.pop_front(), None);
        list.push_back(1);
        list.push_back(2);
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), None);
        assert_eq!(list.pop_back(), None);
    }

    #[test]
    fn test_iter() {
        let mut list = List::new();
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);

        let mut iter = list.iter();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_into_iter() {
        let mut list = List::new();
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);

        let mut iter = list.iter_mut();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }
}
