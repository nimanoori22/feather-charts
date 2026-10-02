//! Synchronous, single-threaded event delegates for the chart engine.
//!
//! `Delegate` intentionally uses `Rc` and `RefCell`: chart-engine events are
//! delivered on the UI thread and are not Iced subscriptions or async streams.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
};

type Callback<E> = Rc<RefCell<Box<dyn FnMut(&E)>>>;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ListenerId(u64);

struct Listener<E> {
    callback: Callback<E>,
    once: bool,
}

struct State<E> {
    next_id: u64,
    listeners: BTreeMap<ListenerId, Listener<E>>,
    emitting: bool,
}

impl<E> Default for State<E> {
    fn default() -> Self {
        Self {
            next_id: 0,
            listeners: BTreeMap::new(),
            emitting: false,
        }
    }
}

/// A synchronous event source owned by a chart-engine component.
///
/// A delegate owns its listener state. [`Subscription`]s only weakly refer to
/// that state, so they neither keep the delegate alive nor create an ownership
/// cycle with their subscriber.
pub struct Delegate<E> {
    state: Rc<RefCell<State<E>>>,
}

impl<E> Default for Delegate<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Delegate<E> {
    /// Creates an empty delegate.
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(State::default())),
        }
    }
}

impl<E: 'static> Delegate<E> {
    /// Registers a callback until its returned subscription is dropped.
    pub fn subscribe(&self, callback: impl FnMut(&E) + 'static) -> Subscription<E> {
        self.subscribe_inner(callback, false)
    }

    /// Registers a callback that is removed before its first invocation.
    pub fn subscribe_once(&self, callback: impl FnMut(&E) + 'static) -> Subscription<E> {
        self.subscribe_inner(callback, true)
    }

    /// Registers a callback bound to a weakly held owner.
    ///
    /// If the owner has been dropped by the time an event is emitted, the
    /// callback deliberately does nothing.
    pub fn subscribe_weak<T: 'static>(
        &self,
        owner: &Rc<RefCell<T>>,
        mut handler: impl FnMut(&mut T, &E) + 'static,
    ) -> Subscription<E> {
        let owner = Rc::downgrade(owner);

        self.subscribe(move |event| {
            let Some(owner) = owner.upgrade() else {
                return;
            };

            handler(&mut owner.borrow_mut(), event);
        })
    }

    /// Registers a weakly bound callback that is removed before first use.
    pub fn subscribe_weak_once<T: 'static>(
        &self,
        owner: &Rc<RefCell<T>>,
        mut handler: impl FnMut(&mut T, &E) + 'static,
    ) -> Subscription<E> {
        let owner = Rc::downgrade(owner);

        self.subscribe_once(move |event| {
            let Some(owner) = owner.upgrade() else {
                return;
            };

            handler(&mut owner.borrow_mut(), event);
        })
    }

    /// Emits an event to the listeners present when this call begins.
    ///
    /// One-shot listeners are removed before callbacks run. Callbacks may add
    /// or remove subscriptions during emission; such changes affect only later
    /// emissions. Re-entering this same delegate is intentionally unsupported.
    pub fn emit(&self, event: &E) {
        let callbacks = {
            let mut state = self.state.borrow_mut();

            assert!(
                !state.emitting,
                "re-entrant emission on the same Delegate is not supported"
            );
            state.emitting = true;

            let callbacks = state
                .listeners
                .values()
                .map(|listener| Rc::clone(&listener.callback))
                .collect::<Vec<_>>();

            // Match Lightweight Charts' Delegate.fire: one-shot listeners are
            // absent from state before any listener observes the event.
            state.listeners.retain(|_, listener| !listener.once);

            callbacks
        };

        let _emission = EmissionGuard {
            state: Rc::downgrade(&self.state),
        };

        for callback in callbacks {
            let mut callback = callback.borrow_mut();
            callback(event);
        }
    }

    /// Returns whether the delegate currently has registered listeners.
    pub fn has_listeners(&self) -> bool {
        !self.state.borrow().listeners.is_empty()
    }

    /// Removes every listener while retaining the delegate itself.
    pub fn clear(&self) {
        self.state.borrow_mut().listeners.clear();
    }

    fn subscribe_inner(&self, callback: impl FnMut(&E) + 'static, once: bool) -> Subscription<E> {
        let mut state = self.state.borrow_mut();
        let id = ListenerId(state.next_id);
        state.next_id += 1;
        state.listeners.insert(
            id,
            Listener {
                callback: Rc::new(RefCell::new(Box::new(callback))),
                once,
            },
        );

        Subscription {
            state: Rc::downgrade(&self.state),
            id,
            active: true,
        }
    }
}

struct EmissionGuard<E> {
    state: Weak<RefCell<State<E>>>,
}

impl<E> Drop for EmissionGuard<E> {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            state.borrow_mut().emitting = false;
        }
    }
}

/// A removable listener registration returned by [`Delegate::subscribe`].
///
/// Dropping this value detaches its listener. It is safe to drop after the
/// source delegate has already been dropped.
pub struct Subscription<E> {
    state: Weak<RefCell<State<E>>>,
    id: ListenerId,
    active: bool,
}

impl<E> Subscription<E> {
    /// Detaches this listener immediately.
    pub fn unsubscribe(mut self) {
        self.detach();
    }

    fn detach(&mut self) {
        if !self.active {
            return;
        }

        self.active = false;

        if let Some(state) = self.state.upgrade() {
            state.borrow_mut().listeners.remove(&self.id);
        }
    }
}

impl<E> Drop for Subscription<E> {
    fn drop(&mut self) {
        self.detach();
    }
}

/// Temporary event marker for the future pane-pointer event payload.
///
/// The corresponding chart input types have not been ported yet. Keeping this
/// concrete marker lets the grouping type below establish its intended
/// ownership shape now; it can gain the real pane-pointer fields when those
/// types arrive.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PanePointerEvent;

/// The two subscriptions a chart widget owns for one pane widget.
///
/// A future `ChartWidget` can keep these in `Vec<PaneEventSubscriptions>` and
/// remove a pair when its corresponding pane is removed. Dropping this struct
/// drops both subscriptions and therefore detaches both listeners.
pub struct PaneEventSubscriptions {
    pub clicked: Subscription<PanePointerEvent>,
    pub double_clicked: Subscription<PanePointerEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        panic,
        rc::Rc,
    };

    #[test]
    fn subscribe_then_emit_runs_callback() {
        let delegate = Delegate::new();
        let received = Rc::new(Cell::new(0));
        let received_by_callback = Rc::clone(&received);
        let _subscription = delegate.subscribe(move |event: &i32| {
            received_by_callback.set(received_by_callback.get() + event);
        });

        delegate.emit(&3);

        assert_eq!(received.get(), 3);
    }

    #[test]
    fn dropping_subscription_stops_delivery() {
        let delegate = Delegate::new();
        let calls = Rc::new(Cell::new(0));
        let calls_by_callback = Rc::clone(&calls);
        let subscription = delegate.subscribe(move |_: &()| {
            calls_by_callback.set(calls_by_callback.get() + 1);
        });

        delegate.emit(&());
        drop(subscription);
        delegate.emit(&());

        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn dropping_subscription_after_delegate_is_safe() {
        let subscription = {
            let delegate = Delegate::<()>::new();
            delegate.subscribe(|_| {})
        };

        drop(subscription);
    }

    #[test]
    fn subscribe_during_emit_waits_until_next_emit() {
        let delegate = Delegate::<()>::new();
        let calls = Rc::new(Cell::new(0));
        let calls_by_new_listener = Rc::clone(&calls);
        let new_subscription = Rc::new(RefCell::new(None));
        let new_subscription_by_callback = Rc::clone(&new_subscription);
        let state = Rc::downgrade(&delegate.state);

        let _outer_subscription = delegate.subscribe(move |_| {
            let delegate = Delegate {
                state: state.upgrade().expect("delegate is alive during emit"),
            };
            let calls = Rc::clone(&calls_by_new_listener);
            let subscription = delegate.subscribe(move |_| calls.set(calls.get() + 1));
            *new_subscription_by_callback.borrow_mut() = Some(subscription);
        });

        delegate.emit(&());
        assert_eq!(calls.get(), 0);

        delegate.emit(&());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn unsubscribe_during_emit_may_still_run_snapshotted_listener() {
        let delegate = Delegate::<()>::new();
        let calls = Rc::new(Cell::new(0));
        let calls_by_listener = Rc::clone(&calls);
        let later_subscription = Rc::new(RefCell::new(None));
        let later_subscription_by_remover = Rc::clone(&later_subscription);

        let _remover = delegate.subscribe(move |_| {
            later_subscription_by_remover.borrow_mut().take();
        });
        let listener = delegate.subscribe(move |_| {
            calls_by_listener.set(calls_by_listener.get() + 1);
        });
        *later_subscription.borrow_mut() = Some(listener);

        delegate.emit(&());
        delegate.emit(&());

        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn subscribe_once_fires_once_and_is_removed_before_the_second_emit() {
        let delegate = Delegate::<()>::new();
        let calls = Rc::new(Cell::new(0));
        let calls_by_callback = Rc::clone(&calls);
        let _subscription = delegate.subscribe_once(move |_| {
            calls_by_callback.set(calls_by_callback.get() + 1);
        });

        delegate.emit(&());
        assert!(!delegate.has_listeners());
        delegate.emit(&());

        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn cross_delegate_emit_is_allowed() {
        let delegate_a = Rc::new(Delegate::<i32>::new());
        let delegate_b = Delegate::<()>::new();
        let calls = Rc::new(Cell::new(0));
        let calls_by_a = Rc::clone(&calls);
        let _a_subscription = delegate_a.subscribe(move |event| {
            calls_by_a.set(calls_by_a.get() + event);
        });
        let delegate_a_from_b = Rc::clone(&delegate_a);
        let _b_subscription = delegate_b.subscribe(move |_| delegate_a_from_b.emit(&7));

        delegate_b.emit(&());

        assert_eq!(calls.get(), 7);
    }

    #[test]
    fn has_listeners_tracks_subscription_drop_and_one_shot_removal() {
        let delegate = Delegate::<()>::new();
        assert!(!delegate.has_listeners());

        let subscription = delegate.subscribe(|_| {});
        assert!(delegate.has_listeners());
        subscription.unsubscribe();
        assert!(!delegate.has_listeners());

        let _one_shot = delegate.subscribe_once(|_| {});
        assert!(delegate.has_listeners());
        delegate.emit(&());
        assert!(!delegate.has_listeners());
    }

    #[test]
    fn dropping_pane_event_subscription_group_detaches_both_listeners() {
        let clicked = Delegate::<PanePointerEvent>::new();
        let double_clicked = Delegate::<PanePointerEvent>::new();
        let calls = Rc::new(Cell::new(0));

        let clicked_calls = Rc::clone(&calls);
        let double_clicked_calls = Rc::clone(&calls);
        let group = PaneEventSubscriptions {
            clicked: clicked.subscribe(move |_| clicked_calls.set(clicked_calls.get() + 1)),
            double_clicked: double_clicked
                .subscribe(move |_| double_clicked_calls.set(double_clicked_calls.get() + 1)),
        };

        drop(group);
        clicked.emit(&PanePointerEvent);
        double_clicked.emit(&PanePointerEvent);

        assert_eq!(calls.get(), 0);
    }

    #[test]
    #[should_panic(expected = "re-entrant emission on the same Delegate is not supported")]
    fn same_delegate_reentrancy_panics_clearly() {
        let delegate = Delegate::<()>::new();
        let state = Rc::downgrade(&delegate.state);
        let _subscription = delegate.subscribe(move |_| {
            let delegate = Delegate {
                state: state.upgrade().expect("delegate is alive during emit"),
            };
            delegate.emit(&());
        });

        delegate.emit(&());
    }

    #[test]
    fn weak_subscriptions_do_not_keep_owners_alive() {
        let delegate = Delegate::<i32>::new();
        let owner = Rc::new(RefCell::new(0));
        let subscription = delegate.subscribe_weak(&owner, |total, event| *total += event);

        delegate.emit(&2);
        assert_eq!(*owner.borrow(), 2);
        drop(owner);
        delegate.emit(&3);

        drop(subscription);
    }

    #[test]
    fn weak_one_shot_subscription_is_removed_before_a_second_emit() {
        let delegate = Delegate::<()>::new();
        let owner = Rc::new(RefCell::new(0));
        let _subscription = delegate.subscribe_weak_once(&owner, |total, _| *total += 1);

        delegate.emit(&());
        delegate.emit(&());

        assert_eq!(*owner.borrow(), 1);
        assert!(!delegate.has_listeners());
    }

    #[test]
    fn panic_during_listener_resets_reentrancy_state() {
        let delegate = Delegate::<()>::new();
        let _subscription = delegate.subscribe(|_| panic!("listener panic"));

        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| delegate.emit(&())));
        delegate.clear();
        let _replacement = delegate.subscribe(|_| {});
        delegate.emit(&());
    }
}
