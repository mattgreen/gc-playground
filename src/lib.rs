use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::{Rc, Weak};

pub struct Heap<T: Trace<T>> {
    objects: RefCell<HashMap<ObjectId, Rc<Header<T>>>>,
    next_id: Cell<ObjectId>,
    collect_threshold: usize,
}

pub type ObjectId = usize;

pub struct Root<T: Trace<T>> {
    inner: Rc<Header<T>>,
}

pub struct Gc<T>(Weak<Header<T>>);

pub trait Trace<T> {
    fn trace(&self, tracer: &mut Tracer<T>);
}

pub struct Tracer<T> {
    objs: Vec<Gc<T>>,
}

struct Header<T> {
    marked: Cell<bool>,
    obj: T,
}

impl<T: Trace<T>> Heap<T> {
    pub fn new(collect_threshold: usize) -> Self {
        Self {
            objects: HashMap::new().into(),
            next_id: 0.into(),
            collect_threshold,
        }
    }

    pub fn allocate(&self, t: T) -> Root<T> {
        let id = self.next_id.get();
        self.next_id.set(id + 1);

        if (id % self.collect_threshold) == 0 {
            self.collect();
        }

        let header = Rc::new(Header {
            marked: false.into(),
            obj: t,
        });

        self.objects.borrow_mut().insert(id, header.clone());

        Root { inner: header }
    }

    pub fn collect(&self) -> usize {
        let mut objects = self.objects.borrow_mut();
        if objects.is_empty() {
            return 0;
        }

        let starting_count = objects.len();

        // Drop all obvious garbage, e.g. objects that have no roots and have no Gc's referring to them.
        // The Heap contains strong refs to all objects, so they won't be removed on their own.
        loop {
            let count = objects.len();
            objects.retain(|_, header| Rc::strong_count(header) > 1 || Rc::weak_count(&header) > 0);

            if objects.len() == count {
                break;
            }
        }

        // Build root set
        // TODO: this could be maintained without scanning all objects
        let roots = objects
            .iter()
            .filter(|(_, header)| Rc::strong_count(header) > 1)
            .map(|(_, header)| Gc(Rc::downgrade(&header)))
            .collect();

        self.mark(roots);
        self.sweep(&mut objects);

        starting_count - objects.len()
    }

    fn mark(&self, roots: Vec<Gc<T>>) {
        let mut tracer: Tracer<T> = Tracer { objs: roots };

        while let Some(gc) = tracer.objs.pop() {
            let header = gc.0.upgrade().unwrap();
            if header.marked() {
                continue;
            }

            header.mark();
            gc.trace(&mut tracer);
        }
    }

    fn sweep(&self, objects: &mut HashMap<ObjectId, Rc<Header<T>>>) {
        objects.retain(|_, header| header.marked());

        for (_, header) in objects.iter_mut() {
            header.clear();
        }
    }

    pub fn object_count(&self) -> usize {
        self.objects.borrow().len()
    }
}

impl<T: Trace<T>> Tracer<T> {
    pub fn trace(&mut self, gc: &Gc<T>) {
        self.objs.push(gc.clone());
    }
}

impl<T: Trace<T>> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let root = self.0.upgrade().expect("object should still be alive");
        let ptr: *const T = &root.obj;

        unsafe { &*ptr }
    }
}

impl<T: Trace<T>> AsRef<T> for Root<T> {
    fn as_ref(&self) -> &T {
        &self.inner.obj
    }
}

impl<T: Trace<T>> Deref for Root<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.obj
    }
}

impl<T: Trace<T>> Clone for Gc<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Trace<T>> From<Root<T>> for Gc<T> {
    fn from(root: Root<T>) -> Self {
        root.to_gc()
    }
}

impl<T: Trace<T>> Root<T> {
    pub fn as_gc(&self) -> Gc<T> {
        Gc(Rc::downgrade(&self.inner))
    }

    pub fn to_gc(self) -> Gc<T> {
        self.as_gc()
    }
}

impl<T: Trace<T>> Header<T> {
    fn clear(&self) {
        self.marked.set(false);
    }

    fn mark(&self) {
        self.marked.set(true);
    }

    fn marked(&self) -> bool {
        self.marked.get() == true
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{Gc, Heap, Trace, Tracer};

    enum Object {
        Cons(RefCell<Gc<Object>>),
        Nil,
    }

    impl Trace<Object> for Object {
        fn trace(&self, tracer: &mut Tracer<Object>) {
            match self {
                Object::Cons(r) => {
                    let obj = r.borrow();
                    tracer.trace(&obj);
                }
                Object::Nil => {}
            }
        }
    }

    #[test]
    fn test_allocate() {
        let heap: Heap<Object> = Heap::new(32);
        let _nil = heap.allocate(Object::Nil);

        assert_eq!(heap.object_count(), 1);
    }

    #[test]
    fn test_roots_survive_collects() {
        let heap: Heap<Object> = Heap::new(32);
        let _nil = heap.allocate(Object::Nil);
        heap.collect();

        assert_eq!(heap.object_count(), 1);

        let _nil2 = heap.allocate(Object::Nil);
        heap.collect();

        assert_eq!(heap.object_count(), 2);
    }

    #[test]
    fn test_unrooted_gcs_do_not_survive() {
        let heap: Heap<Object> = Heap::new(32);
        let _nil = heap.allocate(Object::Nil).to_gc();
        heap.collect();

        assert_eq!(heap.object_count(), 0);
    }

    #[test]
    fn test_rooted_gcs_survive() {
        let heap: Heap<Object> = Heap::new(32);
        let nil = heap.allocate(Object::Nil);
        let _a = heap.allocate(Object::Cons(nil.to_gc().into()));
        heap.collect();

        assert!(heap.object_count() == 2);
    }

    #[test]
    fn test_collects_cycle() {
        let heap: Heap<Object> = Heap::new(32);

        {
            let nil = heap.allocate(Object::Nil);
            let a = heap.allocate(Object::Cons(nil.to_gc().into()));
            assert!(heap.object_count() == 2);

            let nil = heap.allocate(Object::Nil);
            let b = heap.allocate(Object::Cons(nil.to_gc().into()));
            assert!(heap.object_count() == 4);

            if let Object::Cons(b) = b.as_ref() {
                *b.borrow_mut() = a.as_gc();
            }

            if let Object::Cons(a) = a.as_ref() {
                *a.borrow_mut() = b.as_gc();
            }

            heap.collect();

            assert_eq!(heap.object_count(), 2);
        }
        heap.collect();

        assert_eq!(heap.object_count(), 0);
    }
}
