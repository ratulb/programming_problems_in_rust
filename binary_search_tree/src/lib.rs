use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
#[derive(Debug, Default)]
//Default is for the case when we delete a node. We get the value out of the deleted node by
//pushing a default value into it.

struct Node<T: Ord + Default + Clone + std::fmt::Debug> {
    key: T,
    left: Option<Rc<RefCell<Tree<T>>>>,
    right: Option<Rc<RefCell<Tree<T>>>>,
    parent: Option<Weak<RefCell<Node<T>>>>,
}

impl<T: Ord + Default + Clone + std::fmt::Debug> Node<T> {
    //New up a bare node
    fn new(value: T) -> Self {
        Self {
            key: value,
            left: None,
            right: None,
            parent: None,
        }
    }
    //Create a new node wrapped in a RefCell which in turn is
    //wrapped in a Rc
    fn wrapped_node(value: T) -> Option<Rc<RefCell<Self>>> {
        Some(Rc::new(RefCell::new(Node::new(value))))
    }

    //Get a reference to `Node` key
    fn key(&self) -> &T {
        &self.key
    }
    //Does this node has a left child tree
    fn has_left(&self) -> bool {
        self.left.is_some()
    }

    //Does this Tree rooted at this has a right child tree
    fn has_right(&self) -> bool {
        self.right.is_some()
    }
    //Get a shared handle to the root of left child tree
    fn left_node(&self) -> Option<Rc<RefCell<Node<T>>>> {
        self.left
            .as_ref()
            .and_then(|tree| tree.borrow().0.as_ref().map(Rc::clone))
    }
    //Is the given key has the same value as the left tree root node
    //Used when deleting nodes from the tree
    fn is_left_child(&self, key: &T) -> bool {
        Self::left_node(self)
            .as_ref()
            .map(|node| node.borrow().key() == key)
            .unwrap_or(false)
    }

    //Get a shared handle to the right tree's root node
    fn right_node(&self) -> Option<Rc<RefCell<Node<T>>>> {
        self.right
            .as_ref()
            .and_then(|tree| tree.borrow().0.as_ref().map(Rc::clone))
    }

    //Node's parent is a weak reference that we initialize when the node
    //is inserted to the tree - we need to upgrade it to a strong reference
    //to get to the underlying parent node
    fn upgrade_parent(&self) -> Option<Rc<RefCell<Node<T>>>> {
        self.parent.as_ref().and_then(|weak| weak.upgrade())
    }

    //Replace this node's key with the value might be there inside the input
    //Used during delete. If this node is being deleted, then this node's key
    //is flushed out with minimum node's value that is on the right side of
    //this node
    fn replace_key(&mut self, key: Option<T>) -> Option<T> {
        key.map(|k| std::mem::replace(&mut self.key, k))
    }

    //To avoid already borrowed error - if Rc<RefCell> pointing to same location
    fn right_parent<'a>(
        this: Option<&'a Rc<RefCell<Node<T>>>>,
        that: Option<&'a Rc<RefCell<Node<T>>>>,
    ) -> Option<&'a Rc<RefCell<Node<T>>>> {
        match (this, that) {
            (None, None) => None,
            (Some(_), None) => this,
            (None, Some(_)) => that,
            (Some(this_one), Some(that_one)) => match Rc::ptr_eq(this_one, that_one) {
                true => this,
                false => that,
            },
        }
    }

    //Clone this parent which is a weak reference
    //Used during deletion of a node. When the minimum node is taken
    //out from the right side of the node being, the minimum node's
    //right tree(if any) - has to be hoisted up to point to the minimum
    //node's parent
    fn parent(&self) -> Option<Weak<RefCell<Node<T>>>> {
        self.parent.as_ref().map(Weak::clone)
    }

    //Delete a node with single child or no child but node being deleted has parent
    //left: bool -> Should we delete left or right child?
    fn delete_child(&mut self, left: bool) -> Option<T> {
        //First take out the left or right child based on the flag passed in
        let deleted = match left {
            true => self.left.take(),
            false => self.right.take(),
        };
        //result is tuple of the form as shown below
        //result = (deleted.key, deleted.parent, left or right child of deleted)
        let result = match deleted
            .and_then(|tree| tree.take().0)
            .map(|node| node.take())
            .map(|node| (node.key, node.parent, node.left.or(node.right)))
        {
            //Set deleted node's left or right child's parent to the parent of deleted
            Some((key, parent, mut tree)) => {
                if let Some(ref mut inner) = tree {
                    if let Some(ref mut tree_node) = inner.borrow_mut().0 {
                        tree_node.borrow_mut().parent = parent;
                    }
                }
                //(deleted.key, left or right of deleted
                (Some(key), tree)
            }
            None => (None, None),
        };
        //Set self left right to deleted left or right
        match left {
            true => self.left = result.1,
            false => self.right = result.1,
        }
        //deleted key
        result.0
    }

    //Delete a target node - gets invoked when the target node has both left
    //and right node
    fn delete(mut target: Option<Rc<RefCell<Node<T>>>>) -> Option<T> {
        //Find the min node in the right side of the target node that is being
        //deleted
        let min = target
            .as_ref()
            .and_then(|target| target.borrow().right_node().as_ref().and_then(Tree::min));
        //Find strong reference(upgradded from weak) to min's parent
        let min_parent = min.as_ref().and_then(|min| min.borrow().upgrade_parent());
        //Find the right child of min if any. Once min is taken out to fill the
        //deleted target node's content with evicted min node's content, min's right should
        //be pointing at min's parent
        let mut min_right_child = min.as_ref().and_then(|min| {
            min.borrow_mut()
                .right
                .take()
                .as_ref()
                .and_then(|child| child.borrow().root())
        });
        //Make min's right point to min's parent
        if let Some(ref mut child_node) = min_right_child {
            child_node.borrow_mut().parent = min
                .as_ref()
                .and_then(|min| min.borrow().parent().as_ref().cloned());
        }
        //min's parent could be the target node being deleted or some other node on the far
        //right of it. Choose the appropriate parent
        let mut right_parent = Node::right_parent(target.as_ref(), min_parent.as_ref());
        //Set min's right as the right tree of min's parent
        if let Some(ref mut parent) = right_parent {
            parent.borrow_mut().right =
                min_right_child.map(|right_child| Rc::new(RefCell::new(Tree(Some(right_child)))));
        }
        //Return the key of the target node being deleted
        match target {
            Some(ref mut target) => target
                .borrow_mut()
                .replace_key(min.map(|min| min.take().key)),
            None => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct Tree<T: Ord + Default + Clone + std::fmt::Debug>(Option<Rc<RefCell<Node<T>>>>);

impl<T: Ord + Default + Clone + std::fmt::Debug> Tree<T> {
    //Initialize a new tree with the value
    pub fn new(value: T) -> Self {
        Tree(Some(Rc::new(RefCell::new(Node::new(value)))))
    }
    //Create new tree rooted at the input node
    fn new_branch(node: Node<T>) -> Option<Rc<RefCell<Tree<T>>>> {
        Some(Rc::new(RefCell::new(Tree(Some(Rc::new(RefCell::new(
            node,
        )))))))
    }
    //Get a shared handle to the root of the tree
    fn root(&self) -> Option<Rc<RefCell<Node<T>>>> {
        self.0.as_ref().map(Rc::clone)
    }

    //Find the min - given a node. Result could be given node itself
    //if no more left branch is there
    fn min(node: &Rc<RefCell<Node<T>>>) -> Option<Rc<RefCell<Node<T>>>> {
        match node.borrow().left_node() {
            Some(ref left_node) => Self::min(left_node),
            None => Some(Rc::clone(node)),
        }
    }

    //Populate tree with a new key
    //Duplicate keys are not added
    //Recursively calls itself to find place to add the supplied key
    pub fn insert(&mut self, value: T) {
        match self.0 {
            Some(ref mut curr_tree_root) => {
                let mut node = curr_tree_root.borrow_mut();
                if node.key > value {
                    match node.left {
                        Some(ref mut tree) => Self::insert(&mut tree.borrow_mut(), value),
                        None => {
                            let parent = Some(Rc::downgrade(&Rc::clone(curr_tree_root)));
                            let mut left = Node::new(value);
                            left.parent = parent;
                            node.left = Tree::new_branch(left);
                        }
                    }
                } else if node.key < value {
                    match node.right {
                        Some(ref mut tree) => Self::insert(&mut tree.borrow_mut(), value),
                        None => {
                            let parent = Some(Rc::downgrade(&Rc::clone(curr_tree_root)));
                            let mut right = Node::new(value);
                            right.parent = parent;
                            node.right = Tree::new_branch(right);
                        }
                    }
                }
            }
            None => self.0 = Node::wrapped_node(value),
        }
    }

    //Find the node containing the supplied key reference
    fn find(&self, key: &T) -> Option<Rc<RefCell<Node<T>>>> {
        match self.0 {
            Some(ref node) if node.borrow().key() == key => Some(Rc::clone(node)),
            Some(ref node) if node.borrow().key() > key => match node.borrow().left {
                Some(ref left) => Self::find(&left.borrow(), key),
                None => None,
            },
            Some(ref node) if node.borrow().key() < key => match node.borrow().right {
                Some(ref right) => Self::find(&right.borrow(), key),
                None => None,
            },
            Some(_) => None, //Make the compiler happy
            None => None,
        }
    }

    //Delete a node with key that equals the supplied key
    //Returns the deleted key as Some(key) or None otherwise
    pub fn delete(&mut self, key: &T) -> Option<T> {
        let target = Self::find(self, key);
        match target {
            None => None,
            Some(ref node) => {
                let has_left = node.borrow().has_left();
                let has_right = node.borrow().has_right();

                let has_both = has_left && has_right;
                let no_child = !has_left && !has_right;
                let has_parent = node.borrow().parent.is_some();
                match has_parent {
                    false => {
                        //Delete root - root has no parent ref - hence differential treatment
                        match (no_child, has_left, has_right, has_both) {
                            (true, false, false, false) => {
                                self.0.take().map(|root| root.take().key)
                            }
                            //Has left child - remove left child's parent ref and set it as
                            //tree root
                            (false, true, false, false) => {
                                let root = self.root().take();
                                self.0 = root.as_ref().and_then(|root| {
                                    root.borrow().left_node().map(|node| {
                                        node.borrow_mut().parent.take();
                                        node
                                    })
                                });
                                //Return root's key
                                root.map(|root| root.take().key)
                            }
                            //Has right child - remove right child's parent ref and set it as
                            //tree root
                            (false, false, true, false) => {
                                let root = self.root().take();
                                self.0 = root.as_ref().and_then(|root| {
                                    root.borrow().right_node().map(|node| {
                                        node.borrow_mut().parent.take();
                                        node
                                    })
                                });
                                //Return root's key
                                root.map(|root| root.take().key)
                            }
                            //Has got both children - delete to Node::delete
                            (false, true, true, true) => Node::delete(target),
                            (_, _, _, _) => None,
                        }
                    }
                    //target node being deleted has got a parent
                    true => match (no_child, has_left, has_right, has_both) {
                        (true, false, false, false)
                        | (false, true, false, false)
                        | (false, false, true, false) => {
                            let parent = node.borrow().upgrade_parent();
                            //Is it left or right? If no child then left is false
                            let left = parent
                                .as_ref()
                                .map_or(false, |parent| parent.borrow().is_left_child(key));
                            //Delefate to node.delete_child with boolean flag left
                            parent.and_then(|parent| parent.borrow_mut().delete_child(left))
                        }
                        //Has parent, has two children
                        //Delegate to Node::delete
                        (false, true, true, true) => Node::delete(target),
                        (_, _, _, _) => None,
                    },
                }
            }
        }
    }
    //Returns the minimum key value (if any) or `None` otherwise
    //Delegates to internal min function
    pub fn minimum(&self) -> Option<T> {
        let node = self.root();
        match node {
            None => None,
            Some(ref inner) => Self::min(inner).map(|n| n.borrow().key.clone()),
        }
    }
    //Does a key exists in the tree?
    pub fn exists(&self, key: &T) -> bool {
        match self.0 {
            Some(ref node) => {
                node.borrow().key() == key || {
                    let in_left = match node.borrow().left {
                        Some(ref tree) => Self::exists(&tree.borrow(), key),
                        None => false,
                    };

                    let in_right = match node.borrow().right {
                        Some(ref tree) => Self::exists(&tree.borrow(), key),
                        None => false,
                    };
                    in_left || in_right
                }
            }
            None => false,
        }
    }

    //Does this contains the other tree?
    pub fn contains(&self, other: &Self) -> bool {
        match self {
            Tree(None) => match other {
                Tree(_) => false,
            },
            Tree(Some(ref this)) => match other {
                Tree(None) => true,
                that @ Tree(_) => {
                    if Self::is_identical(self, that) {
                        return true;
                    }
                    let left_contains = match this.borrow().left {
                        Some(ref tree) => Self::contains(&tree.borrow(), that),
                        None => false,
                    };
                    let right_contains = match this.borrow().right {
                        Some(ref tree) => Self::contains(&tree.borrow(), that),
                        None => false,
                    };
                    left_contains || right_contains
                }
            },
        }
    }

    //Is this tree is identical to other tree?
    pub fn is_identical(&self, other: &Self) -> bool {
        match self.0 {
            Some(ref this) => match other {
                Tree(Some(ref that)) => {
                    if this.borrow().key == that.borrow().key {
                        let this_left = &this.borrow().left;
                        let that_left = &that.borrow().left;
                        let this_right = &this.borrow().right;
                        let that_right = &that.borrow().right;
                        let left_matched = match this_left {
                            Some(ref this_tree) => match that_left {
                                Some(ref that_tree) => {
                                    return Self::is_identical(
                                        &this_tree.borrow(),
                                        &that_tree.borrow(),
                                    );
                                }
                                None => false,
                            },
                            None => that_left.is_none(),
                        };
                        let right_matched = match this_right {
                            Some(ref this_tree) => match that_right {
                                Some(ref that_tree) => {
                                    return Self::is_identical(
                                        &this_tree.borrow(),
                                        &that_tree.borrow(),
                                    );
                                }
                                None => false,
                            },
                            None => that_right.is_none(),
                        };
                        left_matched && right_matched
                    } else {
                        false
                    }
                }
                Tree(None) => false,
            },

            None => match other {
                Tree(Some(_)) => false,
                Tree(None) => true,
            },
        }
    }

    //Find the height of the tree
    pub fn height(&self) -> usize {
        let root = self.root();
        match root {
            None => 0,
            Some(ref node)
                if node.borrow().left_node().is_none() & node.borrow().right_node().is_none() =>
            {
                1
            }
            Some(ref node) => {
                let left_tree_height = node
                    .borrow()
                    .left
                    .as_ref()
                    .map(|tree| Self::height(&tree.borrow()))
                    .unwrap_or(0);
                let right_tree_height = node
                    .borrow()
                    .right
                    .as_ref()
                    .map(|tree| Self::height(&tree.borrow()))
                    .unwrap_or(0);
                1 + std::cmp::max(left_tree_height, right_tree_height)
            }
        }
    }
    //Return the lowest common ancestor for two given keys
    pub fn lowest_common_ancestor(&self, this: &T, that: &T) -> Option<T> {
        if let Some(ref root) = self.root() {
            let root = root.borrow();
            if root.key() < this && root.key() < that {
                if let Some(ref right) = root.right {
                    return Self::lowest_common_ancestor(&right.borrow(), this, that);
                } else {
                    return None;
                }
            } else if root.key() > this && root.key() > that {
                if let Some(ref left) = root.left {
                    return Self::lowest_common_ancestor(&left.borrow(), this, that);
                } else {
                    return None;
                }
            } else {
                return Some(root.key().clone());
            }
        } else {
            None
        }
    }

    //Get an iterator for the tree's keys
    //Remember - calling iter on the tree would not consume the tree
    //iterator.next would return Option<T>
    //T is cloned
    //Keys would would be returned level wise
    pub fn iter(&self) -> Iter<T> {
        Iter {
            next: self.root().map(|node| {
                let mut next = VecDeque::new();
                next.push_front(node);
                next
            }),
        }
    }

    //Returns an iterator that consumes the tree elements one by one
    //when calling next on it
    //Root of the tree is eviced when next is called on the iterator
    pub fn into_iter(&mut self) -> IntoIter<'_, T> {
        IntoIter {
            tree: match self {
                Tree(None) => None,
                Tree(_) => Some(self),
            },
        }
    }
}

#[derive(Debug)]
pub struct Iter<T: Ord + Default + Clone + std::fmt::Debug> {
    next: Option<VecDeque<Rc<RefCell<Node<T>>>>>,
}

impl<T: Ord + Default + Clone + std::fmt::Debug> Iterator for Iter<T> {
    type Item = T;
    //Level wise iterator
    fn next(&mut self) -> Option<Self::Item> {
        match self.next {
            None => None,
            Some(ref mut queue) => {
                let popped = queue.pop_back();
                match popped {
                    None => None,
                    Some(ref node) => {
                        let node = node.borrow();
                        if let Some(ref left) = node.left_node() {
                            queue.push_front(Rc::clone(left));
                        }
                        if let Some(ref right) = node.right_node() {
                            queue.push_front(Rc::clone(right));
                        }
                        Some(node.key.clone())
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct IntoIter<'a, T: Ord + Default + Clone + std::fmt::Debug> {
    tree: Option<&'a mut Tree<T>>,
}

impl<T: Ord + Default + Clone + std::fmt::Debug> Iterator for IntoIter<'_, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        match self.tree {
            None => None,
            Some(ref mut tree) => match tree.0 {
                None => None,
                Some(ref mut node) => {
                    let key = node.borrow().key.clone();
                    tree.delete(&key);
                    Some(key)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical() {
        let mut tree1 = Tree::new(1);
        tree1.insert(2);
        tree1.insert(3);
        assert!(!tree1.is_identical(&Tree(None)));
        let mut tree2 = Tree::new(1);
        assert!(!tree1.is_identical(&tree2));
        tree2.insert(2);
        tree2.insert(3);
        assert!(tree1.is_identical(&tree2));
        assert!(Tree::new(None::<String>).is_identical(&Tree::new(None)));
    }

    #[test]
    fn test_contains() {
        let mut tree = Tree::new(42);
        tree.insert(24);
        tree.insert(40);
        tree.insert(35);
        assert!(tree.contains(&Tree(None)));
        assert!(tree.contains(&Tree::new(35)));
        assert!(!tree.contains(&Tree::new(40)));

        let mut subtree = Tree::new(24);
        subtree.insert(40);
        subtree.insert(35);
        assert!(tree.contains(&subtree));
        assert!(!subtree.contains(&Tree::new(24)));
    }

    #[test]
    fn test_exists() {
        let mut tree = Tree::new(42);
        tree.insert(24);
        tree.insert(40);
        tree.insert(35);
        assert!(tree.exists(&42));
        assert!(tree.exists(&24));
        assert!(tree.exists(&40));
        assert!(tree.exists(&35));
        assert!(!tree.exists(&100));
    }

    #[test]
    fn test_find() {
        let mut tree = Tree::new(42);
        tree.insert(24);
        tree.insert(40);
        tree.insert(35);
        tree.insert(200);
        assert!(tree.find(&200).is_some());
        assert!(tree.find(&42).is_some());
        assert!(tree.find(&24).is_some());
        assert!(tree.find(&40).is_some());
        assert!(tree.find(&35).is_some());
        assert!(tree.find(&100).is_none());
    }

    #[test]
    fn test_min() {
        let mut tree = Tree::new(42);
        tree.insert(24);
        tree.insert(40);
        tree.insert(35);
        tree.insert(5);
        assert_eq!(Tree::min(&tree.0.unwrap()).unwrap().take().key, 5);
    }

    #[test]
    fn test_delete_child() {
        let mut tree = Tree::new(42);
        tree.insert(24);
        tree.insert(40);
        tree.insert(35);
        tree.insert(50);
        assert!(tree.find(&24).is_some());
        tree.0.as_mut().unwrap().borrow_mut().delete_child(true);
        assert!(tree.find(&24).is_none());
    }

    #[test]
    fn test_delete() {
        let mut tree = Tree::new(42);
        let result = tree.delete(&42);
        assert!(tree.find(&42).is_none());
        assert_eq!(result, Some(42));
        //Left only tree
        let mut tree = Tree::new(3);
        tree.insert(2);
        tree.insert(1);
        let result = tree.delete(&3);
        assert!(tree.find(&3).is_none());
        assert_eq!(result, Some(3));
        let result = tree.delete(&2);
        assert_eq!(result, Some(2));
        //Right only tree
        let mut tree = Tree::new(1);
        tree.insert(2);
        tree.insert(3);
        let result = tree.delete(&1);
        assert!(tree.find(&1).is_none());
        assert_eq!(result, Some(1));
        let result = tree.delete(&2);
        assert_eq!(result, Some(2));
    }
    #[test]
    fn delete_root_with_both_subtrees() {
        //Right and left tree - evict root
        let mut tree = Tree::new(20);
        tree.insert(10);
        tree.insert(30);
        tree.insert(25);
        let result = tree.delete(&20);
        assert_eq!(result, Some(20));
    }

    #[test]
    fn delete_root_with_both_subtree_1_level() {
        //Right and left tree - evict root
        let mut tree = Tree::new(20);
        tree.insert(10);
        tree.insert(30);
        let result = tree.delete(&20);
        assert_eq!(result, Some(20));
    }
    #[test]
    fn delete_node_with_parent_no_child() {
        //Right and left tree - evict root
        let mut tree = Tree::new(20);
        tree.insert(10);
        tree.insert(30);
        let result = tree.delete(&10);
        assert_eq!(result, Some(10));
        assert!(tree.find(&10).is_none());

        let result = tree.delete(&30);
        assert_eq!(result, Some(30));
        assert!(tree.find(&30).is_none());
    }

    #[test]
    fn delete_node_with_parent_one_child() {
        //Right and left tree - evict root
        let mut tree = Tree::new(25);
        tree.insert(10);
        tree.insert(15);
        tree.insert(20);
        tree.insert(30);
        let result = tree.delete(&10);
        assert_eq!(result, Some(10));
        assert!(tree.find(&10).is_none());

        assert!(tree.find(&30).is_some());
        let result = tree.delete(&30);
        assert_eq!(result, Some(30));
        assert!(tree.find(&30).is_none());

        let result = tree.delete(&25);
        assert_eq!(result, Some(25));
        assert!(tree.find(&25).is_none());
        let result = tree.delete(&20);
        assert_eq!(result, Some(20));
        assert!(tree.find(&20).is_none());
        let result = tree.delete(&15);
        assert_eq!(result, Some(15));
        assert!(tree.find(&15).is_none());
        assert!(tree.0.is_none())
    }

    #[test]
    fn delete_node_with_parent_both_childrent() {
        let mut tree = Tree::new(25);
        tree.insert(10);
        tree.insert(15);
        tree.insert(20);
        tree.insert(5);
        let result = tree.delete(&10);
        assert_eq!(result, Some(10));
        assert!(tree.find(&10).is_none());

        let result = tree.delete(&25);
        assert_eq!(result, Some(25));
        assert!(tree.find(&25).is_none());

        let result = tree.delete(&20);
        assert_eq!(result, Some(20));
        assert!(tree.find(&20).is_none());

        let result = tree.delete(&5);
        assert_eq!(result, Some(5));
        assert!(tree.find(&5).is_none());

        let result = tree.delete(&15);
        assert_eq!(result, Some(15));
        assert!(tree.find(&15).is_none());

        let result = tree.delete(&15);
        assert_eq!(result, None);
        assert!(tree.find(&15).is_none());
    }

    #[test]
    fn minimum_test() {
        let mut tree = Tree::new(25);
        tree.insert(10);
        tree.insert(15);
        tree.insert(20);
        tree.insert(5);
        assert_eq!(tree.minimum(), Some(5));
        let _ = tree.delete(&5);
        assert_eq!(tree.minimum(), Some(10));
        let _ = tree.delete(&10);
        assert_eq!(tree.minimum(), Some(15));
        let _ = tree.delete(&15);
        assert_eq!(tree.minimum(), Some(20));
        let _ = tree.delete(&20);
        assert_eq!(tree.minimum(), Some(25));
        let _ = tree.delete(&25);
        assert_eq!(tree.minimum(), None);
    }

    #[test]
    fn iter_test() {
        let mut tree = Tree::new(25);
        tree.insert(10);
        tree.insert(15);
        tree.insert(20);
        tree.insert(5);
        let mut iter = tree.iter();
        assert_eq!(iter.next(), Some(25));
        assert_eq!(iter.next(), Some(10));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(15));
        assert_eq!(iter.next(), Some(20));
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn itermut_test() {
        let mut tree = Tree::new(25);
        tree.insert(10);
        tree.insert(15);
        tree.insert(20);
        tree.insert(5);
        let mut iter = tree.into_iter();
        assert_eq!(iter.next(), Some(25));
        assert_eq!(iter.next(), Some(10));
        assert_eq!(iter.next(), Some(15));
        assert_eq!(iter.next(), Some(20));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_delete1() {
        let mut tree = Tree::new(25);

        tree.insert(10);
        tree.insert(15);
        tree.insert(20);
        tree.insert(5);
        let node15 = tree.root().and_then(|root| {
            root.borrow()
                .left_node()
                .and_then(|left| left.borrow().right_node())
        });
        let deleted = Node::delete(node15);
        assert_eq!(deleted, Some(15));
    }

    #[test]
    fn test_delete_2() {
        let mut tree = Tree::new(25);
        tree.insert(10);
        tree.insert(15);
        tree.insert(20);
        tree.insert(5);
        let result = tree.delete(&10);
        assert_eq!(result, Some(10));
        let result = tree.delete(&15);
        assert_eq!(result, Some(15));
        let result = tree.delete(&25);
        assert_eq!(result, Some(25));
    }

    #[test]
    fn test_delete_3() {
        let mut tree = Tree::new(27);
        tree.insert(18);
        tree.insert(24);
        tree.insert(21);
        tree.insert(25);
        tree.insert(30);
        let result = tree.delete(&24);
        assert_eq!(result, Some(24));
        let result = tree.delete(&21);
        assert_eq!(result, Some(21));
        let result = tree.delete(&27);
        assert_eq!(result, Some(27));
    }

    #[test]
    fn test_height() {
        let mut tree = Tree::new(1);
        assert_eq!(tree.height(), 1);
        tree.delete(&1);
        assert_eq!(tree.height(), 0);
        tree.insert(1);
        tree.insert(2);
        tree.insert(3);
        assert_eq!(tree.height(), 3);
        tree.insert(-1);
        tree.insert(-2);
        assert_eq!(tree.height(), 3);
        tree.insert(-3);
        assert_eq!(tree.height(), 4);
    }
    #[test]
    fn test_lowest_common_ancestor() {
        let mut tree = Tree::new(6);
        tree.insert(2);
        tree.insert(8);
        tree.insert(0);
        tree.insert(4);
        tree.insert(7);
        tree.insert(9);
        tree.insert(3);
        tree.insert(5);
        assert_eq!(tree.lowest_common_ancestor(&3, &5), Some(4));
        assert_eq!(tree.lowest_common_ancestor(&2, &5), Some(2));
        assert_eq!(tree.lowest_common_ancestor(&0, &5), Some(2));
        assert_eq!(tree.lowest_common_ancestor(&3, &7), Some(6));
    }
}
