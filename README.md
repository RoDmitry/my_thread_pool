# My own thread pool implementation (draft)

Faster than `Rayon` or `Threadpool`, but close to `Threadpool` 2.0 with `Crossbeam` channel.

Has `Scope` implementation, but any outside lifetimes in a closure of `Scope::send` will not be checked (unsafe). So be sure that any borrowed values live at least until `Scope::drop`, or it will lead to an UB (crash).

> Note: `Box<dyn FnOnce()>` (`TaskBoxed`) makes it slower, so you better use a static function, intead of a dynamic one.
