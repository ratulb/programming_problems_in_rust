#![forbid(unsafe_code)]
use std::cell::RefCell;
use std::fmt::{Debug, Error, Formatter};
use std::rc::Rc;

type Cell<T> = Rc<RefCell<Node<T>>>;
type Link<T> = Option<Cell<T>>;

#[derive(PartialEq)]
pub(crate) struct Node<T> {
    elem: T,
    next: Link<T>,
}

///
///The link list structure for arbritary type T. 'T' should have a default value.
///

#[derive(PartialEq)]
pub struct LinkedList<T> {
    head: Link<T>,
    len: usize,
}

impl<T: Default> Node<T> {
    pub fn new(elem: T) -> Self {
        Self {
            elem: elem,
            next: None,
        }
    }

    fn with_link(elem: T, link: Cell<T>) -> Cell<T> {
        Rc::new(RefCell::new(Self {
            elem: elem,
            next: Some(link),
        }))
    }

    //Push to the back of the node chain
    fn push_back(&mut self, elem: T) {
        match self.next {
            None => self.next = Some(Self::rc_cell(elem)),
            Some(ref mut next) => next.borrow_mut().push_back(elem),
        }
    }

    //Would pop the last node in the chain. Would stop at 'this'. No way to self sabotage in rust!
    fn pop_back(&mut self) -> Option<T> {
        match self.next {
            None => None,
            Some(ref mut next) => {
                if next.borrow().next.is_none() {
                    let result = Some(next.borrow_mut().take());
                    let _ = self.next.take();
                    return result;
                } else {
                    //Would blow out the stack for last list//TODO revisit
                    //On a second thought - its ok since not exposed to wider world!
                    return next.borrow_mut().pop_back();
                }
            }
        }
    }
    #[inline(always)]
    fn rc_cell(elem: T) -> Cell<T> {
        Rc::new(RefCell::new(Self::new(elem)))
    }

    //Takes out the value from the node. Replaces it with the default value for type 'T'.
    fn take(&mut self) -> T {
        std::mem::replace(&mut self.elem, T::default())
    }
}

impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self.next {
            None => write!(f, "{:?}", self.elem),
            Some(ref next) => {
                let _ = write!(f, "{:?} -> ", self.elem);
                next.borrow().fmt(f)
            }
        }
    }
}

impl<T: Default> LinkedList<T> {
    //Creates a list with a single value
    pub fn new(elem: T) -> Self {
        Self {
            head: Some(Node::rc_cell(elem)),
            len: 1,
        }
    }

    //Readily create a list from clonable slice of values. Internally values are never cloned hereafter.
    pub fn from_slice<U: Clone + Default>(elems: &[U]) -> LinkedList<U> {
        assert!(elems.len() > 0);
        let mut list = LinkedList::<U>::default();
        elems.iter().for_each(|elem| list.push_back(elem.clone()));
        list
    }
    //Push value to the front of the list
    pub fn push_front(&mut self, elem: T) {
        match self.head.take() {
            Some(as_link) => self.head = Some(Node::with_link(elem, as_link)),
            None => self.head = Some(Node::rc_cell(elem)),
        }
        self.len += 1;
    }

    //Pop value out from the front of the list - O(1) operation
    pub fn pop_front(&mut self) -> Option<T> {
        self.head.take().map(|head| {
            self.len -= 1;
            self.head = head.borrow_mut().next.take();
            head.borrow_mut().take()
        })
    }
    //Push values to the back of the list. O(n) recursive operation
    pub fn push_back(&mut self, elem: T) {
        if self.is_empty() {
            self.push_front(elem);
        } else {
            let mut last = self
                .iterator()
                .enumerate()
                .skip_while(|(index, _)| index != &(self.len() - 1))
                .map(|t| t.1)
                .next();

            if let Some(ref mut last) = last {
                last.borrow_mut().next = Some(Node::rc_cell(elem));
                self.len += 1;
            }
        }
    }

    //Pop values from the end of the list
    pub fn pop_back(&mut self) -> Option<T> {
        if self.head.is_none() {
            return None;
        } else if self.len() == 1 {
            self.len -= 1;
            self.head.take().map(|head| head.borrow_mut().take())
        } else {
            let penultimate = self
                .iterator()
                .enumerate()
                .skip_while(|(index, _)| index != &(self.len() - 2))
                .map(|t| t.1)
                .next();

            penultimate.and_then(|penultimate| {
                penultimate.borrow_mut().next.take().map(|last| {
                    self.len -= 1;
                    last.borrow_mut().take()
                })
            })
        }
    }
    //Count of values in the list
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    //Reverse the list
    pub fn reverse(&mut self) {
        if self.len < 2 {
            return;
        }
        let mut previous = None;
        let mut current = self.head.take();
        while let Some(ref mut curr_node) = current {
            let mut curr_next = curr_node.borrow_mut().next.take();
            curr_node.borrow_mut().next = previous.take();
            previous = current.take();
            current = curr_next.take();
            if current.is_none() {
                break;
            }
        }
        self.head = previous;
    }

    pub(crate) fn iterator(&self) -> LinkIterator<T> {
        LinkIterator {
            links: self.head.as_ref().map(Rc::clone),
        }
    }
}
//Default linked list contains nothing
impl<T> Default for LinkedList<T> {
    fn default() -> Self {
        Self { head: None, len: 0 }
    }
}
//An iterator used internally
pub(crate) struct LinkIterator<T> {
    links: Link<T>,
}

impl<T> Iterator for LinkIterator<T> {
    type Item = Cell<T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.links.take().map(|link| {
            //Following two are equivalent - top one would increase Rc count
            //self.links = link.borrow_mut().next.take().as_ref().map(Rc::clone);
            // self.links = link.borrow_mut().next.take();
            //The following would not dissociate returned link from next
            self.links = link.borrow().next.as_ref().map(Rc::clone);
            link
        })
    }
}

impl<T: Debug> Debug for LinkedList<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self.head {
            None => write!(f, "Empty linked list"),
            Some(ref node) => {
                let _ = write!(f, "{}", "{");
                let _ = node.borrow().fmt(f);
                let _ = write!(f, "{}", "}, size: ");
                write!(f, "{}", self.len)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linkedlist_link_iterator_test_1() {
        let elems = (1..5).collect::<Vec<_>>();
        let list = LinkedList::<i32>::from_slice(&elems);
        let itr = list.iterator();
        let mut elem = 1;
        for link in itr {
            assert_eq!(link.borrow_mut().take(), elem);
            elem += 1;
        }
    }

    #[test]
    fn linkedlist_size_test_1() {
        let elems = (1..21750).collect::<Vec<_>>();
        let mut list = LinkedList::<i32>::from_slice(&elems);
        list.reverse();
        let elems = (1..21750).rev().collect::<Vec<_>>();
        let reversed = LinkedList::<i32>::from_slice(&elems);
        // println!("The reversed list = {:?}", list);
        assert_eq!(list, reversed);
    }

    #[test]
    fn linkedlist_reverse_test_1() {
        let elems = [100, 200, 300, 400, 500];
        let mut list = LinkedList::<i32>::from_slice(&elems);
        list.reverse();
        let elems = [500, 400, 300, 200, 100];
        let reversed = LinkedList::<i32>::from_slice(&elems);
        assert_eq!(list, reversed);
    }

    #[test]
    fn list_push_front_test_1() {
        let mut list = LinkedList::default();
        list.push_front(3);
        list.push_front(2);
        list.push_front(1);
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), Some(3));
        assert_eq!(list.pop_front(), None);
        assert_eq!(list.pop_back(), None);
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn node_pop_back_test_1() {
        let mut node = Node::new(1);
        node.push_back(2);
        node.push_back(3);
        node.push_back(4);
        assert_eq!(node.pop_back(), Some(4));
        assert_eq!(node.pop_back(), Some(3));
        assert_eq!(node.pop_back(), Some(2));
        assert_eq!(node.pop_back(), None);
    }

    #[test]
    fn linkedlist_pop_back_test_1() {
        let elems = (1..21750).collect::<Vec<_>>();
        let mut list = LinkedList::<i32>::from_slice(&elems);
        for num in (1..21750).rev() {
            assert_eq!(list.pop_back(), Some(num as i32));
        }
        assert_eq!(list.pop_back(), None);
    }
}

///
///Implentation of a singly linked list with cossuming, ref and mutable iterator
///
pub mod iterable {
    use std::fmt::{Debug, Error, Formatter};
    use std::rc::Rc;

    type Link<T> = Option<Rc<Node<T>>>;
    struct Node<T> {
        elem: T,
        next: Link<T>,
    }

    pub struct LinkedList<T> {
        head: Link<T>,
        len: usize,
    }

    impl<T: Default> Node<T> {
        pub fn new(elem: T) -> Self {
            Self {
                elem: elem,
                next: None,
            }
        }

        fn with_link(elem: T, link: Rc<Node<T>>) -> Rc<Node<T>> {
            Rc::new(Self {
                elem: elem,
                next: Some(link),
            })
        }

        pub fn push_back(&mut self, elem: T) {
            match self.next {
                None => self.next = Some(Rc::new(Self::new(elem))),
                Some(ref mut next) => {
                    if let Some(node) = Rc::get_mut(next) {
                        node.push_back(elem);
                    }
                }
            }
        }
        //Would pop until this node
        pub fn pop_back(&mut self) -> Option<T> {
            match self.next {
                None => None,
                Some(ref mut next) => {
                    let next_is_none = next.next.is_none();
                    if let Some(node) = Rc::get_mut(next) {
                        if next_is_none {
                            let result = Some(node.take());
                            let _ = self.next.take();
                            return result;
                        } else {
                            return node.pop_back();
                        }
                    }
                    None
                }
            }
        }

        /***pub fn push_front(&mut self, elem: T) {
            *self = Self {
                elem: elem,
                next: Some(Rc::new(std::mem::take(self))),
            };
        }***/

        fn take(&mut self) -> T {
            std::mem::replace(&mut self.elem, T::default())
        }
    }

    impl<T: Default> Default for Node<T> {
        fn default() -> Self {
            Self {
                elem: T::default(),
                next: None,
            }
        }
    }

    impl<T: PartialEq> Eq for Node<T> {}
    impl<T: PartialEq> PartialEq for Node<T> {
        fn eq(&self, other: &Self) -> bool {
            match (&self.elem, &self.next, &other.elem, &other.next) {
                (elem, _, other_elem, _) if *elem != *other_elem => false,
                (elem, None, other_elem, None) if *elem == *other_elem => true,
                (_, None, _, Some(_)) => false,
                (_, Some(_), _, None) => false,
                (elem, Some(this), other_elem, Some(that)) if *elem == *other_elem => this.eq(that),
                (_, _, _, _) => false,
            }
        }
    }

    impl<T: Debug> Debug for Node<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
            match self.next {
                None => write!(f, "{:?}", self.elem),
                Some(ref next) => {
                    let _ = write!(f, "{:?} -> ", self.elem);
                    next.fmt(f)
                }
            }
        }
    }

    impl<T: Default> LinkedList<T> {
        pub fn new(elem: T) -> Self {
            Self {
                head: Some(Rc::new(Node::new(elem))),
                len: 1,
            }
        }

        //Create a list from from clonable types
        pub fn from_slice<U: Clone + Default>(elems: &[U]) -> LinkedList<U> {
            assert!(elems.len() > 0);
            let mut node = Node::<U>::new(elems[0].clone());
            elems[1..]
                .iter()
                .for_each(|elem| node.push_back(elem.clone()));

            LinkedList {
                head: Some(Rc::new(node)),
                len: elems.len(),
            }
        }

        pub fn push_front(&mut self, elem: T) {
            match self.head.take() {
                Some(as_link) => self.head = Some(Node::with_link(elem, as_link)),
                None => self.head = Some(Rc::new(Node::new(elem))),
            }
            self.len += 1;
        }

        pub fn push_back(&mut self, elem: T) {
            match self.head.as_mut().and_then(Rc::get_mut) {
                None => self.head = Some(Rc::new(Node::new(elem))),
                Some(node) => node.push_back(elem),
            }
            self.len += 1;
        }

        pub fn pop_front(&mut self) -> Option<T> {
            match self.head.take() {
                Some(taken) => match Rc::into_inner(taken) {
                    None => None,
                    Some(mut node) => {
                        self.head = node.next.take();
                        self.len -= 1;
                        Some(node.take())
                    }
                },
                None => None,
            }
        }

        pub fn pop_back(&mut self) -> Option<T> {
            if self.head.is_none() {
                return None;
            }
            if let Some(head) = self.head.as_mut() {
                if let Some(node) = Rc::get_mut(head) {
                    if self.len == 1 {
                        let result = Some(node.take());
                        self.len -= 1;
                        let _ = self.head.take();
                        return result;
                    } else {
                        let result = node.pop_back();
                        if result.is_some() {
                            self.len -= 1;
                        }
                        return result;
                    }
                }
            }
            None
        }

        pub fn len(&self) -> usize {
            self.len
        }

        //Update a value at the given index
        pub fn update(&mut self, index: usize, elem: T) -> Option<T> {
            if index >= self.len {
                return None;
            }
            self.iter_mut()
                .enumerate()
                .skip_while(|(idx, _)| idx != &index)
                .take(1)
                .next()
                .map(|t| t.1)
                .map(|t| std::mem::replace(t, elem))
        }

        pub fn reverse(&mut self) {
            if self.len < 2 {
                return;
            }
            let mut previous = None;
            let mut current = self.head.take();
            while let Some(ref mut curr_inner) = current.as_mut().and_then(Rc::get_mut) {
                let next = curr_inner.next.take();
                curr_inner.next = previous.take();
                previous = current.take();
                current = next;
            }
            self.head = previous;
        }

        pub fn iter(&self) -> Iter<'_, T> {
            Iter {
                link: self.head.as_ref().map(|rc_node| rc_node.as_ref()),
            }
        }

        ///Returns a mut iterator for the elements of the list. Mutating the
        ///elems would change the backing list
        pub fn iter_mut(&mut self) -> IterMut<'_, T> {
            IterMut {
                link: self.head.as_mut().and_then(Rc::get_mut),
            }
        }

        pub fn into_iter(self) -> IntoIter<T> {
            let mut head = self.head;
            IntoIter {
                link: head.take().and_then(Rc::into_inner),
            }
        }
    }

    impl<T> Default for LinkedList<T> {
        fn default() -> Self {
            Self { head: None, len: 0 }
        }
    }

    impl<T: Debug> Debug for LinkedList<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
            match self.head {
                None => write!(f, "Empty linked list"),
                Some(ref node) => {
                    let _ = write!(f, "{}", "{");
                    let _ = node.fmt(f);
                    let _ = write!(f, "{}", "}, size: ");
                    write!(f, "{}", self.len)
                }
            }
        }
    }

    impl<T: PartialEq> Eq for LinkedList<T> {}
    impl<T: PartialEq> PartialEq for LinkedList<T> {
        fn eq(&self, other: &Self) -> bool {
            self.len == other.len && self.head == other.head
        }
    }

    pub struct IntoIter<T> {
        link: Option<Node<T>>,
    }

    impl<T> Iterator for IntoIter<T> {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            self.link.take().map(|node| {
                self.link = node.next.and_then(Rc::into_inner);
                node.elem
            })
        }
    }

    pub struct Iter<'a, T> {
        link: Option<&'a Node<T>>,
    }

    impl<'a, T> Iterator for Iter<'a, T> {
        type Item = &'a T;
        fn next(&mut self) -> Option<Self::Item> {
            self.link.map(|node| {
                self.link = node.next.as_ref().map(|next| next.as_ref()); //next = &Rc<Node<T>>
                                                                          //self.node = node.next.as_ref().map(|next| next.deref());
                                                                          //self.node = node.next.as_deref();
                &node.elem
            })
        }
    }

    pub struct IterMut<'a, T> {
        link: Option<&'a mut Node<T>>,
    }

    impl<'a, T> Iterator for IterMut<'a, T> {
        type Item = &'a mut T;
        fn next(&mut self) -> Option<Self::Item> {
            self.link.take().map(|node| {
                self.link = node.next.as_mut().and_then(|next| Rc::get_mut(next)); //next = &mut Rc<Node<T>>
                &mut node.elem
            })
        }
    }
    #[cfg(test)]
    mod iterable_tests {
        use super::*;
        #[test]
        fn linklist_iter_test() {
            let elems = [1, 2, 3, 4, 5];
            let list = LinkedList::<i32>::from_slice(&elems);
            let mut iter = list.iter();
            for num in 1..=5 {
                assert_eq!(iter.next(), Some(&num as &i32));
            }
            assert_eq!(iter.next(), None);
        }
        #[test]
        fn linklist_iter_mut_test() {
            let elems = [1, 2, 3, 4, 5];
            let mut list = LinkedList::<i32>::from_slice(&elems);
            let mut iter = list.iter_mut();
            for _ in 0..5 {
                if let Some(elem) = iter.next() {
                    *elem *= 100;
                }
            }
            let elems = [100, 200, 300, 400, 500];
            let expected = LinkedList::<i32>::from_slice(&elems);
            assert_eq!(list, expected);
        }

        #[test]
        fn linklist_into_iter_test() {
            let elems = [1, 2, 3, 4, 5];
            let list = LinkedList::<i32>::from_slice(&elems);
            let mut iter = list.into_iter();
            for num in 1..=5 {
                assert_eq!(iter.next(), Some(num));
            }
            assert_eq!(iter.next(), None);
        }
        #[test]
        fn linklist_push_back_test() {
            let mut list = LinkedList::<i32>::default();
            list.push_back(1);
            list.push_back(2);
            list.push_back(3);
            let mut iter = list.into_iter();
            for num in 1..=3 {
                assert_eq!(iter.next(), Some(num));
            }
            assert_eq!(iter.next(), None);

            let mut list = LinkedList::<i32>::new(1);
            list.push_back(2);
            list.push_back(3);
            let mut iter = list.into_iter();
            for num in 1..=3 {
                assert_eq!(iter.next(), Some(num));
            }
            assert_eq!(iter.next(), None);
        }

        #[test]
        fn node_pop_back_test_2() {
            let mut node = Node::new(1);
            node.push_back(2);
            node.push_back(3);
            node.push_back(4);
            println!("Node is: {:?}", node);
            assert_eq!(node.pop_back(), Some(4));
            assert_eq!(node.pop_back(), Some(3));
            assert_eq!(node.pop_back(), Some(2));
            assert_eq!(node.pop_back(), None);
            println!("Now node is: {:?}", node);
        }

        #[test]
        fn linkedlist_pop_back_test_2() {
            let mut list = LinkedList::new(1);
            list.push_back(2);
            list.push_back(3);
            list.push_back(4);
            println!("List is: {:?}", list);
            assert_eq!(list.pop_back(), Some(4));
            assert_eq!(list.pop_back(), Some(3));
            assert_eq!(list.pop_back(), Some(2));
            assert_eq!(list.pop_back(), Some(1));
            assert_eq!(list.pop_back(), None);
            println!("Now list is: {:?}", list);
        }

        #[test]
        fn linkedlist_reverse_test_2() {
            let elems = [100, 200, 300, 400, 500];
            let mut list = LinkedList::<i32>::from_slice(&elems);
            list.reverse();
            let elems = [500, 400, 300, 200, 100];
            let reversed = LinkedList::<i32>::from_slice(&elems);
            assert_eq!(list, reversed);
        }

        #[test]
        fn linkedlist_update_test() {
            let elems = [100, 200, 300, 400, 500];
            let mut list = LinkedList::<i32>::from_slice(&elems);
            let result = list.update(4, 1000);
            assert_eq!(result, Some(500));
            println!("The updated list is: {:?}", list);

            let result = list.update(0, 999);
            assert_eq!(result, Some(100));
            println!("The updated list is: {:?}", list);
        }
    }
}