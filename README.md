# Rust GC playground

Recently, I've become interested in garbage collection algorithms.

To learn them, I've created a toy garbage collector, re-creating [rcgc](https://github.com/jonas-schievink/rcgc/) in the process. Because `rcgc` implements a classical garbage collector in terms of reference-counting, it remains small, easy to understand, and is mostly safe code. My version ended up slightly differently; I still haven't decided if I'm wrong on some parts or not.

Currently, it is a single-threaded stop-the-world precise mark and sweep collector.

My wishlist for learning and implementation:

  * [ ] fuzz current implementation and see where it explodes
  * [ ] generational collector
  * [ ] support for multiple threads
  * [ ] move away from `Rc`-based design to a more conventional approach
  * [ ] investigate alternative approaches

## Links

### Rust GC Projects
  * [shifgrethor](https://github.com/withoutboats/shifgrethor)

### Reference Counting Papers
  * [A Unified Theory of Garbage Collection](https://courses.cs.washington.edu/courses/cse590p/05au/p50-bacon.pdf)
  * [An Efficient On-the-Fly Cycle Collection](https://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.85.9708&rep=rep1&type=pdf)
