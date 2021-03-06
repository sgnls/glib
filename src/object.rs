// Copyright 2015-2016, The Gtk-rs Project Developers.
// See the COPYRIGHT file at the top-level directory of this distribution.
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

//! `IMPL` Object wrapper implementation and `Object` binding.

use translate::*;
use types::{self, StaticType};
use wrapper::{UnsafeFrom, Wrapper};
use ffi as glib_ffi;
use gobject_ffi;
use std::mem;
use std::ptr;
use std::iter;
use std::ops;
use std::marker::PhantomData;

use Value;
use value::{ToValue, SetValue};
use Type;
use BoolError;
use Closure;
use SignalHandlerId;

use get_thread_id;

/// Upcasting and downcasting support.
///
/// Provides conversions up and down the class hierarchy tree.
pub trait Cast: IsA<Object> {
    /// Upcasts an object to a superclass or interface `T`.
    ///
    /// *NOTE*: This statically checks at compile-time if casting is possible. It is not always
    /// known at compile-time, whether a specific object implements an interface or not, in which case
    /// `upcast` would fail to compile. `dynamic_cast` can be used in these circumstances, which
    /// is checking the types at runtime.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let button = gtk::Button::new();
    /// let widget = button.upcast::<gtk::Widget>();
    /// ```
    #[inline]
    fn upcast<T>(self) -> T
    where T: StaticType + UnsafeFrom<ObjectRef> + Wrapper,
          Self: IsA<T> {
        unsafe { T::from(self.into()) }
    }

    /// Upcasts an object to a reference of its superclass or interface `T`.
    ///
    /// *NOTE*: This statically checks at compile-time if casting is possible. It is not always
    /// known at compile-time, whether a specific object implements an interface or not, in which case
    /// `upcast` would fail to compile. `dynamic_cast` can be used in these circumstances, which
    /// is checking the types at runtime.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let button = gtk::Button::new();
    /// let widget = button.upcast_ref::<gtk::Widget>();
    /// ```
    #[inline]
    fn upcast_ref<T>(&self) -> &T
    where T: StaticType + UnsafeFrom<ObjectRef> + Wrapper,
          Self: IsA<T> {
        unsafe {
            // This transmute is safe because all our wrapper types have the
            // same representation except for the name and the phantom data
            // type. IsA<> is an unsafe trait that must only be implemented
            // if this is a valid wrapper type
            mem::transmute(self)
        }
    }

    /// Tries to downcast to a subclass or interface implementor `T`.
    ///
    /// Returns `Ok(T)` if the object is an instance of `T` and `Err(self)`
    /// otherwise.
    ///
    /// *NOTE*: This statically checks at compile-time if casting is possible. It is not always
    /// known at compile-time, whether a specific object implements an interface or not, in which case
    /// `upcast` would fail to compile. `dynamic_cast` can be used in these circumstances, which
    /// is checking the types at runtime.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let button = gtk::Button::new();
    /// let widget = button.upcast::<gtk::Widget>();
    /// assert!(widget.downcast::<gtk::Button>().is_ok());
    /// ```
    #[inline]
    fn downcast<T>(self) -> Result<T, Self>
    where Self: Sized + Downcast<T> {
        Downcast::downcast(self)
    }

    /// Tries to downcast to a reference of its subclass or interface implementor `T`.
    ///
    /// Returns `Some(T)` if the object is an instance of `T` and `None`
    /// otherwise.
    ///
    /// *NOTE*: This statically checks at compile-time if casting is possible. It is not always
    /// known at compile-time, whether a specific object implements an interface or not, in which case
    /// `upcast` would fail to compile. `dynamic_cast` can be used in these circumstances, which
    /// is checking the types at runtime.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let button = gtk::Button::new();
    /// let widget = button.upcast::<gtk::Widget>();
    /// assert!(widget.downcast_ref::<gtk::Button>().is_some());
    /// ```
    #[inline]
    fn downcast_ref<T>(&self) -> Option<&T>
    where Self: Sized + Downcast<T> {
        Downcast::downcast_ref(self)
    }

    /// Returns `true` if the object is an instance of (can be cast to) `T`.
    fn is<T>(&self) -> bool
    where T: StaticType {
        unsafe {
            types::instance_of::<T>(self.to_glib_none().0 as *const _)
        }
    }

    /// Tries to cast to an object of type `T`. This handles upcasting, downcasting
    /// and casting between interface and interface implementors. All checks are performed at
    /// runtime, while `downcast` and `upcast` will do many checks at compile-time already.
    ///
    /// It is not always known at compile-time, whether a specific object implements an interface or
    /// not, and checking as to be performed at runtime.
    ///
    /// Returns `Ok(T)` if the object is an instance of `T` and `Err(self)`
    /// otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let button = gtk::Button::new();
    /// let widget = button.dynamic_cast::<gtk::Widget>();
    /// assert!(widget.is_ok());
    /// let widget = widget.unwrap();
    /// assert!(widget.dynamic_cast::<gtk::Button>().is_ok());
    /// ```
    #[inline]
    fn dynamic_cast<T>(self) -> Result<T, Self>
    where T: StaticType + UnsafeFrom<ObjectRef> + Wrapper {
        if !self.is::<T>() {
            Err(self)
        } else {
            Ok(unsafe { T::from(self.into()) })
        }
    }

    /// Tries to cast to reference to an object of type `T`. This handles upcasting, downcasting
    /// and casting between interface and interface implementors. All checks are performed at
    /// runtime, while `downcast` and `upcast` will do many checks at compile-time already.
    ///
    /// It is not always known at compile-time, whether a specific object implements an interface or
    /// not, and checking as to be performed at runtime.
    ///
    /// Returns `Some(T)` if the object is an instance of `T` and `None`
    /// otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let button = gtk::Button::new();
    /// let widget = button.dynamic_cast_ref::<gtk::Widget>();
    /// assert!(widget.is_some());
    /// let widget = widget.unwrap();
    /// assert!(widget.dynamic_cast_ref::<gtk::Button>().is_some());
    /// ```
    #[inline]
    fn dynamic_cast_ref<T>(&self) -> Option<&T>
    where T: StaticType + UnsafeFrom<ObjectRef> + Wrapper {
        if !self.is::<T>() {
            None
        } else {
            // This transmute is safe because all our wrapper types have the
            // same representation except for the name and the phantom data
            // type. IsA<> is an unsafe trait that must only be implemented
            // if this is a valid wrapper type
            Some(unsafe { mem::transmute(self) })
        }
    }
}

impl<T: IsA<Object>> Cast for T { }

/// Declares the "is a" relationship.
///
/// `Self` is said to implement `T`.
///
/// For instance, since originally `GtkWidget` is a subclass of `GObject` and
/// implements the `GtkBuildable` interface, `gtk::Widget` implements
/// `IsA<glib::Object>` and `IsA<gtk::Buildable>`.
///
///
/// The trait can only be implemented if the appropriate `ToGlibPtr`
/// implementations exist.
///
/// `T` always implements `IsA<T>`.
pub unsafe trait IsA<T: StaticType + UnsafeFrom<ObjectRef> + Wrapper>: StaticType + Wrapper +
    Into<ObjectRef> + UnsafeFrom<ObjectRef> +
    for<'a> ToGlibPtr<'a, *mut <T as Wrapper>::GlibType> { }

unsafe impl<T> IsA<T> for T
where T: StaticType + Wrapper + Into<ObjectRef> + UnsafeFrom<ObjectRef> +
    for<'a> ToGlibPtr<'a, *mut <T as Wrapper>::GlibType> { }

/// Trait for mapping a class struct type to its corresponding instance type.
pub unsafe trait IsClassFor: Sized + 'static {
    /// Corresponding Rust instance type for this class.
    type Instance;

    /// Get the type id for this class.
    fn get_type(&self) -> Type {
        unsafe {
            let klass = self as *const _ as *const gobject_ffi::GTypeClass;
            from_glib((*klass).g_type)
        }
    }

    /// Casts this class to a reference to a parent type's class.
    fn upcast_ref<U: IsClassFor>(&self) -> &U
        where Self::Instance: IsA<U::Instance>,
            U::Instance: Wrapper + StaticType + UnsafeFrom<ObjectRef>
    {
        unsafe {
            let klass = self as *const _ as *const U;
            &*klass
        }
    }

    /// Casts this class to a mutable reference to a parent type's class.
    fn upcast_ref_mut<U: IsClassFor>(&mut self) -> &mut U
        where Self::Instance: IsA<U::Instance>,
            U::Instance: Wrapper + StaticType + UnsafeFrom<ObjectRef>
    {
        unsafe {
            let klass = self as *mut _ as *mut U;
            &mut *klass
        }
    }

    /// Casts this class to a reference to a child type's class or
    /// fails if this class is not implementing the child class.
    fn downcast_ref<U: IsClassFor>(&self) -> Option<&U>
        where U::Instance: IsA<Self::Instance>,
            Self::Instance: Wrapper + StaticType + UnsafeFrom<ObjectRef>
    {
        if !self.get_type().is_a(&U::Instance::static_type()) {
            return None;
        }

        unsafe {
            let klass = self as *const _ as *const U;
            Some(&*klass)
        }
    }

    /// Casts this class to a mutable reference to a child type's class or
    /// fails if this class is not implementing the child class.
    fn downcast_ref_mut<U: IsClassFor>(&mut self) -> Option<&mut U>
        where U::Instance: IsA<Self::Instance>,
            Self::Instance: Wrapper + StaticType + UnsafeFrom<ObjectRef>
    {
        if !self.get_type().is_a(&U::Instance::static_type()) {
            return None;
        }

        unsafe {
            let klass = self as *mut _ as *mut U;
            Some(&mut *klass)
        }
    }
}

/// Downcasts support.
pub trait Downcast<T> {
    /// Checks if it's possible to downcast to `T`.
    ///
    /// Returns `true` if the instance implements `T` and `false` otherwise.
    fn can_downcast(&self) -> bool;
    /// Tries to downcast to `T`.
    ///
    /// Returns `Ok(T)` if the instance implements `T` and `Err(Self)` otherwise.
    fn downcast(self) -> Result<T, Self> where Self: Sized;
    /// Tries to downcast to `&T`.
    ///
    /// Returns `Some(T)` if the instance implements `T` and `None` otherwise.
    fn downcast_ref(&self) -> Option<&T>;
    /// Downcasts to `T` unconditionally.
    ///
    /// Panics if compiled with `debug_assertions` and the instance doesn't implement `T`.
    unsafe fn downcast_unchecked(self) -> T;
    /// Downcasts to `&T` unconditionally.
    ///
    /// Panics if compiled with `debug_assertions` and the instance doesn't implement `T`.
    unsafe fn downcast_ref_unchecked(&self) -> &T;
}

impl<Super: IsA<Super>, Sub: IsA<Super>> Downcast<Sub> for Super {
    #[inline]
    fn can_downcast(&self) -> bool {
        unsafe {
            types::instance_of::<Sub>(self.to_glib_none().0 as *const _)
        }
    }

    #[inline]
    fn downcast(self) -> Result<Sub, Super> {
        unsafe {
            if !types::instance_of::<Sub>(self.to_glib_none().0 as *const _) {
                return Err(self);
            }
            Ok(Sub::from(self.into()))
        }
    }

    #[inline]
    fn downcast_ref(&self) -> Option<&Sub> {
        unsafe {
            if !types::instance_of::<Sub>(self.to_glib_none().0 as *const _) {
                return None;
            }
            // This transmute is safe because all our wrapper types have the
            // same representation except for the name and the phantom data
            // type. IsA<> is an unsafe trait that must only be implemented
            // if this is a valid wrapper type
            Some(mem::transmute(self))
        }
    }

    #[inline]
    unsafe fn downcast_unchecked(self) -> Sub {
        debug_assert!(types::instance_of::<Sub>(self.to_glib_none().0 as *const _));
        Sub::from(self.into())
    }

    #[inline]
    unsafe fn downcast_ref_unchecked(&self) -> &Sub {
        debug_assert!(types::instance_of::<Sub>(self.to_glib_none().0 as *const _));
        // This transmute is safe because all our wrapper types have the
        // same representation except for the name and the phantom data
        // type. IsA<> is an unsafe trait that must only be implemented
        // if this is a valid wrapper type
        mem::transmute(self)
    }
}

#[doc(hidden)]
pub use gobject_ffi::GObject;

#[doc(hidden)]
pub use gobject_ffi::GObjectClass;

glib_wrapper! {
    #[doc(hidden)]
    #[derive(Debug, Ord, PartialOrd, PartialEq, Eq, Hash)]
    pub struct ObjectRef(Shared<GObject>);

    match fn {
        ref => |ptr| gobject_ffi::g_object_ref_sink(ptr),
        unref => |ptr| gobject_ffi::g_object_unref(ptr),
    }
}

/// Wrapper implementations for Object types. See `glib_wrapper!`.
#[macro_export]
macro_rules! glib_object_wrapper {
    ([$($attr:meta)*] $name:ident, $ffi_name:path, $ffi_class_name:path, $rust_class_name:path, @get_type $get_type_expr:expr) => {
        $(#[$attr])*
        // Always derive Hash/Ord (and below impl Debug, PartialEq, Eq, PartialOrd) for object
        // types. Due to inheritance and up/downcasting we must implement these by pointer or
        // otherwise they would potentially give differeny results for the same object depending on
        // the type we currently know for it
        #[derive(Clone, Hash, Ord)]
        pub struct $name($crate::object::ObjectRef, ::std::marker::PhantomData<$ffi_name>);

        #[doc(hidden)]
        impl Into<$crate::object::ObjectRef> for $name {
            fn into(self) -> $crate::object::ObjectRef {
                self.0
            }
        }

        #[doc(hidden)]
        impl $crate::wrapper::UnsafeFrom<$crate::object::ObjectRef> for $name {
            unsafe fn from(t: $crate::object::ObjectRef) -> Self {
                $name(t, ::std::marker::PhantomData)
            }
        }

        #[doc(hidden)]
        impl $crate::translate::GlibPtrDefault for $name {
            type GlibType = *mut $ffi_name;
        }

        #[doc(hidden)]
        impl $crate::wrapper::Wrapper for $name {
            type GlibType = $ffi_name;
            type GlibClassType = $ffi_class_name;
            type RustClassType = $rust_class_name;
        }

        #[doc(hidden)]
        impl<'a> $crate::translate::ToGlibPtr<'a, *const $ffi_name> for $name {
            type Storage = <$crate::object::ObjectRef as
                $crate::translate::ToGlibPtr<'a, *mut $crate::object::GObject>>::Storage;

            #[inline]
            fn to_glib_none(&'a self) -> $crate::translate::Stash<'a, *const $ffi_name, Self> {
                let stash = self.0.to_glib_none();
                $crate::translate::Stash(stash.0 as *const _, stash.1)
            }

            #[inline]
            fn to_glib_full(&self) -> *const $ffi_name {
                self.0.to_glib_full() as *const _
            }
        }

        #[doc(hidden)]
        impl<'a> $crate::translate::ToGlibPtr<'a, *mut $ffi_name> for $name {
            type Storage = <$crate::object::ObjectRef as
                $crate::translate::ToGlibPtr<'a, *mut $crate::object::GObject>>::Storage;

            #[inline]
            fn to_glib_none(&'a self) -> $crate::translate::Stash<'a, *mut $ffi_name, Self> {
                let stash = self.0.to_glib_none();
                $crate::translate::Stash(stash.0 as *mut _, stash.1)
            }

            #[inline]
            fn to_glib_full(&self) -> *mut $ffi_name {
                self.0.to_glib_full() as *mut _
            }
        }

        #[doc(hidden)]
        impl<'a> $crate::translate::ToGlibContainerFromSlice<'a, *mut *mut $ffi_name> for $name {
            type Storage = (Vec<Stash<'a, *mut $ffi_name, $name>>, Option<Vec<*mut $ffi_name>>);

            fn to_glib_none_from_slice(t: &'a [$name]) -> (*mut *mut $ffi_name, Self::Storage) {
                let v: Vec<_> = t.iter().map(|s| s.to_glib_none()).collect();
                let mut v_ptr: Vec<_> = v.iter().map(|s| s.0).collect();
                v_ptr.push(ptr::null_mut() as *mut $ffi_name);

                (v_ptr.as_ptr() as *mut *mut $ffi_name, (v, Some(v_ptr)))
            }

            fn to_glib_container_from_slice(t: &'a [$name]) -> (*mut *mut $ffi_name, Self::Storage) {
                let v: Vec<_> = t.iter().map(|s| s.to_glib_none()).collect();

                let v_ptr = unsafe {
                    let v_ptr = glib_ffi::g_malloc0(mem::size_of::<*mut $ffi_name>() * (t.len() + 1)) as *mut *mut $ffi_name;

                    for (i, s) in v.iter().enumerate() {
                        ptr::write(v_ptr.add(i), s.0);
                    }

                    v_ptr
                };

                (v_ptr, (v, None))
            }

            fn to_glib_full_from_slice(t: &[$name]) -> *mut *mut $ffi_name {
                unsafe {
                    let v_ptr = glib_ffi::g_malloc0(mem::size_of::<*mut $ffi_name>() * (t.len() + 1)) as *mut *mut $ffi_name;

                    for (i, s) in t.iter().enumerate() {
                        ptr::write(v_ptr.add(i), s.to_glib_full());
                    }

                    v_ptr
                }
            }
        }

        #[doc(hidden)]
        impl<'a> $crate::translate::ToGlibContainerFromSlice<'a, *const *mut $ffi_name> for $name {
            type Storage = (Vec<Stash<'a, *mut $ffi_name, $name>>, Option<Vec<*mut $ffi_name>>);

            fn to_glib_none_from_slice(t: &'a [$name]) -> (*const *mut $ffi_name, Self::Storage) {
                let (ptr, stash) = $crate::translate::ToGlibContainerFromSlice::<'a, *mut *mut $ffi_name>::to_glib_none_from_slice(t);
                (ptr as *const *mut $ffi_name, stash)
            }

            fn to_glib_container_from_slice(_: &'a [$name]) -> (*const *mut $ffi_name, Self::Storage) {
                // Can't have consumer free a *const pointer
                unimplemented!()
            }

            fn to_glib_full_from_slice(_: &[$name]) -> *const *mut $ffi_name {
                // Can't have consumer free a *const pointer
                unimplemented!()
            }
        }

        #[doc(hidden)]
        impl $crate::translate::FromGlibPtrNone<*mut $ffi_name> for $name {
            #[inline]
            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
            unsafe fn from_glib_none(ptr: *mut $ffi_name) -> Self {
                debug_assert!($crate::types::instance_of::<Self>(ptr as *const _));
                $name($crate::translate::from_glib_none(ptr as *mut _), ::std::marker::PhantomData)
            }
        }

        #[doc(hidden)]
        impl $crate::translate::FromGlibPtrNone<*const $ffi_name> for $name {
            #[inline]
            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
            unsafe fn from_glib_none(ptr: *const $ffi_name) -> Self {
                debug_assert!($crate::types::instance_of::<Self>(ptr as *const _));
                $name($crate::translate::from_glib_none(ptr as *mut _), ::std::marker::PhantomData)
            }
        }

        #[doc(hidden)]
        impl $crate::translate::FromGlibPtrFull<*mut $ffi_name> for $name {
            #[inline]
            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
            unsafe fn from_glib_full(ptr: *mut $ffi_name) -> Self {
                debug_assert!($crate::types::instance_of::<Self>(ptr as *const _));
                $name($crate::translate::from_glib_full(ptr as *mut _), ::std::marker::PhantomData)
            }
        }

        #[doc(hidden)]
        impl $crate::translate::FromGlibPtrBorrow<*mut $ffi_name> for $name {
            #[inline]
            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
            unsafe fn from_glib_borrow(ptr: *mut $ffi_name) -> Self {
                debug_assert!($crate::types::instance_of::<Self>(ptr as *const _));
                $name($crate::translate::from_glib_borrow(ptr as *mut _),
                      ::std::marker::PhantomData)
            }
        }

        #[doc(hidden)]
        impl $crate::translate::FromGlibContainerAsVec<*mut $ffi_name, *mut *mut $ffi_name> for $name {
            unsafe fn from_glib_none_num_as_vec(ptr: *mut *mut $ffi_name, num: usize) -> Vec<Self> {
                if num == 0 || ptr.is_null() {
                    return Vec::new();
                }

                let mut res = Vec::with_capacity(num);
                for i in 0..num {
                    res.push($crate::translate::from_glib_none(ptr::read(ptr.add(i))));
                }
                res
            }

            unsafe fn from_glib_container_num_as_vec(ptr: *mut *mut $ffi_name, num: usize) -> Vec<Self> {
                let res = $crate::translate::FromGlibContainerAsVec::from_glib_none_num_as_vec(ptr, num);
                glib_ffi::g_free(ptr as *mut _);
                res
            }

            unsafe fn from_glib_full_num_as_vec(ptr: *mut *mut $ffi_name, num: usize) -> Vec<Self> {
                if num == 0 || ptr.is_null() {
                    return Vec::new();
                }

                let mut res = Vec::with_capacity(num);
                for i in 0..num {
                    res.push($crate::translate::from_glib_full(ptr::read(ptr.add(i))));
                }
                glib_ffi::g_free(ptr as *mut _);
                res
            }
        }

        #[doc(hidden)]
        impl $crate::translate::FromGlibPtrArrayContainerAsVec<*mut $ffi_name, *mut *mut $ffi_name> for $name {
            unsafe fn from_glib_none_as_vec(ptr: *mut *mut $ffi_name) -> Vec<Self> {
                $crate::translate::FromGlibContainerAsVec::from_glib_none_num_as_vec(ptr, $crate::translate::c_ptr_array_len(ptr))
            }

            unsafe fn from_glib_container_as_vec(ptr: *mut *mut $ffi_name) -> Vec<Self> {
                $crate::translate::FromGlibContainerAsVec::from_glib_container_num_as_vec(ptr, $crate::translate::c_ptr_array_len(ptr))
            }

            unsafe fn from_glib_full_as_vec(ptr: *mut *mut $ffi_name) -> Vec<Self> {
                $crate::translate::FromGlibContainerAsVec::from_glib_full_num_as_vec(ptr, $crate::translate::c_ptr_array_len(ptr))
            }
        }

        #[doc(hidden)]
        impl $crate::translate::FromGlibContainerAsVec<*mut $ffi_name, *const *mut $ffi_name> for $name {
            unsafe fn from_glib_none_num_as_vec(ptr: *const *mut $ffi_name, num: usize) -> Vec<Self> {
                $crate::translate::FromGlibContainerAsVec::from_glib_none_num_as_vec(ptr as *mut *mut _, num)
            }

            unsafe fn from_glib_container_num_as_vec(_: *const *mut $ffi_name, _: usize) -> Vec<Self> {
                // Can't free a *const
                unimplemented!()
            }

            unsafe fn from_glib_full_num_as_vec(_: *const *mut $ffi_name, _: usize) -> Vec<Self> {
                // Can't free a *const
                unimplemented!()
            }
        }

        #[doc(hidden)]
        impl $crate::translate::FromGlibPtrArrayContainerAsVec<*mut $ffi_name, *const *mut $ffi_name> for $name {
            unsafe fn from_glib_none_as_vec(ptr: *const *mut $ffi_name) -> Vec<Self> {
                $crate::translate::FromGlibPtrArrayContainerAsVec::from_glib_none_as_vec(ptr as *mut *mut _)
            }

            unsafe fn from_glib_container_as_vec(_: *const *mut $ffi_name) -> Vec<Self> {
                // Can't free a *const
                unimplemented!()
            }

            unsafe fn from_glib_full_as_vec(_: *const *mut $ffi_name) -> Vec<Self> {
                // Can't free a *const
                unimplemented!()
            }
        }

        impl $crate::types::StaticType for $name {
            fn static_type() -> $crate::types::Type {
                #[allow(unused_unsafe)]
                unsafe { $crate::translate::from_glib($get_type_expr) }
            }
        }

        impl<T: $crate::object::IsA<$crate::object::Object>> ::std::cmp::PartialEq<T> for $name {
            #[inline]
            fn eq(&self, other: &T) -> bool {
                use $crate::translate::ToGlibPtr;
                self.0.to_glib_none().0 == other.to_glib_none().0
            }
        }

        impl ::std::cmp::Eq for $name { }

        impl<T: $crate::object::IsA<$crate::object::Object>> ::std::cmp::PartialOrd<T> for $name {
            #[inline]
            fn partial_cmp(&self, other: &T) -> Option<::std::cmp::Ordering> {
                use $crate::translate::ToGlibPtr;
                self.0.to_glib_none().0.partial_cmp(&other.to_glib_none().0)
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("inner", &self.0)
                    .field("type", &<$name as $crate::ObjectExt>::get_type(self))
                    .finish()
            }
        }

        #[doc(hidden)]
        impl<'a> $crate::value::FromValueOptional<'a> for $name {
            unsafe fn from_value_optional(value: &$crate::Value) -> Option<Self> {
                Option::<$name>::from_glib_full(gobject_ffi::g_value_dup_object(value.to_glib_none().0) as *mut $ffi_name)
                    .map(|o| $crate::object::Downcast::downcast_unchecked(o))
            }
        }

        #[doc(hidden)]
        impl $crate::value::SetValue for $name {
            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
            unsafe fn set_value(value: &mut $crate::Value, this: &Self) {
                gobject_ffi::g_value_set_object(value.to_glib_none_mut().0, $crate::translate::ToGlibPtr::<*mut $ffi_name>::to_glib_none(this).0 as *mut gobject_ffi::GObject)
            }
        }

        #[doc(hidden)]
        impl $crate::value::SetValueOptional for $name {
            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
            unsafe fn set_value_optional(value: &mut $crate::Value, this: Option<&Self>) {
                gobject_ffi::g_value_set_object(value.to_glib_none_mut().0, $crate::translate::ToGlibPtr::<*mut $ffi_name>::to_glib_none(&this).0 as *mut gobject_ffi::GObject)
            }
        }
    };

    (@munch_impls $name:ident, ) => { };

    (@munch_impls $name:ident, $super_name:path) => {
        #[doc(hidden)]
        impl<'a> $crate::translate::ToGlibPtr<'a,
                *mut <$super_name as $crate::wrapper::Wrapper>::GlibType> for $name {
            type Storage = <$crate::object::ObjectRef as
                $crate::translate::ToGlibPtr<'a, *mut $crate::object::GObject>>::Storage;

            #[inline]
            fn to_glib_none(&'a self) -> $crate::translate::Stash<'a,
                    *mut <$super_name as $crate::wrapper::Wrapper>::GlibType, Self> {
                let stash = self.0.to_glib_none();
                unsafe {
                    debug_assert!($crate::types::instance_of::<$super_name>(stash.0 as *const _));
                }
                $crate::translate::Stash(stash.0 as *mut _, stash.1)
            }

            #[inline]
            fn to_glib_full(&self)
                    -> *mut <$super_name as $crate::wrapper::Wrapper>::GlibType {
                let ptr = self.0.to_glib_full();
                unsafe {
                    debug_assert!($crate::types::instance_of::<$super_name>(ptr as *const _));
                }
                ptr as *mut _
            }
        }

        unsafe impl $crate::object::IsA<$super_name> for $name { }
    };

    (@munch_impls $name:ident, $super_name:path => $super_ffi:path) => {
        #[doc(hidden)]
        impl<'a> $crate::translate::ToGlibPtr<'a, *mut $super_ffi> for $name {
            type Storage = <$crate::object::ObjectRef as
                $crate::translate::ToGlibPtr<'a, *mut $crate::object::GObject>>::Storage;

            #[inline]
            fn to_glib_none(&'a self) -> $crate::translate::Stash<'a, *mut $super_ffi, Self> {
                let stash = self.0.to_glib_none();
                unsafe {
                    debug_assert!($crate::types::instance_of::<$super_name>(stash.0 as *const _));
                }
                $crate::translate::Stash(stash.0 as *mut _, stash.1)
            }

            #[inline]
            fn to_glib_full(&self) -> *mut $super_ffi {
                let ptr = self.0.to_glib_full();
                unsafe {
                    debug_assert!($crate::types::instance_of::<$super_name>(ptr as *const _));
                }
                ptr as *mut _
            }
        }

        unsafe impl $crate::object::IsA<$super_name> for $name { }
    };

    (@munch_impls $name:ident, $super_name:path, $($implements:tt)*) => {
        glib_object_wrapper!(@munch_impls $name, $super_name);
        glib_object_wrapper!(@munch_impls $name, $($implements)*);
    };

    (@munch_impls $name:ident, $super_name:path => $super_ffi:path, $($implements:tt)*) => {
        glib_object_wrapper!(@munch_impls $name, $super_name => $super_ffi);
        glib_object_wrapper!(@munch_impls $name, $($implements)*);
    };

    ([$($attr:meta)*] $name:ident, $ffi_name:path, $ffi_class_name:path, $rust_class_name:path,
        @get_type $get_type_expr:expr, @implements $($implements:tt)*) => {
        glib_object_wrapper!([$($attr)*] $name, $ffi_name, $ffi_class_name, $rust_class_name,
            @get_type $get_type_expr);
        glib_object_wrapper!(@munch_impls $name, $($implements)*);

        #[doc(hidden)]
        impl<'a> $crate::translate::ToGlibPtr<'a, *mut $crate::object::GObject> for $name {
            type Storage = <$crate::object::ObjectRef as
                $crate::translate::ToGlibPtr<'a, *mut $crate::object::GObject>>::Storage;

            #[inline]
            fn to_glib_none(&'a self)
                    -> $crate::translate::Stash<'a, *mut $crate::object::GObject, Self> {
                let stash = self.0.to_glib_none();
                $crate::translate::Stash(stash.0 as *mut _, stash.1)
            }

            #[inline]
            fn to_glib_full(&self) -> *mut $crate::object::GObject {
                (&self.0).to_glib_full() as *mut _
            }
        }

        unsafe impl $crate::object::IsA<$crate::object::Object> for $name { }
    };

    ([$($attr:meta)*] $name:ident, $ffi_name:path, $ffi_class_name:path, $rust_class_name:path, @get_type $get_type_expr:expr,
     [$($implements:path),*]) => {
        glib_object_wrapper!([$($attr)*] $name, $ffi_name, $ffi_class_name, $rust_class_name,
            @get_type $get_type_expr, @implements $($implements),*);
    };
}

glib_object_wrapper! {
    [doc = "The base class in the object hierarchy."]
    Object, GObject, GObjectClass, ObjectClass, @get_type gobject_ffi::g_object_get_type()
}

impl Object {
    pub fn new(type_: Type, properties: &[(&str, &ToValue)]) -> Result<Object, BoolError> {
        use std::ffi::CString;

        if !type_.is_a(&Object::static_type()) {
            return Err(BoolError("Can't instantiate non-GObject objects"));
        }

        let params = properties.iter()
                               .map(|&(name, value)|
                                    (CString::new(name).unwrap(), value.to_value()))
                               .collect::<Vec<_>>();

        let params_c = params.iter()
                             .map(|&(ref name, ref value)|
                                  gobject_ffi::GParameter {
                                      name: name.as_ptr(),
                                      value: unsafe { *value.to_glib_none().0 }
                                  })
                             .collect::<Vec<_>>();

        unsafe {
            let ptr = gobject_ffi::g_object_newv(type_.to_glib(), params_c.len() as u32, mut_override(params_c.as_ptr()));
            if ptr.is_null() {
                Err(BoolError("Can't instantiate object"))
            } else if gobject_ffi::g_object_is_floating(ptr) != glib_ffi::GFALSE {
                Ok(from_glib_none(ptr))
            } else {
                Ok(from_glib_full(ptr))
            }
        }
    }
}

pub trait ObjectExt: IsA<Object> {
    fn get_type(&self) -> Type;
    fn get_object_class(&self) -> &ObjectClass;

    fn set_property<'a, N: Into<&'a str>>(&self, property_name: N, value: &ToValue) -> Result<(), BoolError>;
    fn get_property<'a, N: Into<&'a str>>(&self, property_name: N) -> Result<Value, BoolError>;
    fn has_property<'a, N: Into<&'a str>>(&self, property_name: N, type_: Option<Type>) -> Result<(), BoolError>;
    fn get_property_type<'a, N: Into<&'a str>>(&self, property_name: N) -> Option<Type>;
    fn find_property<'a, N: Into<&'a str>>(&self, property_name: N) -> Option<::ParamSpec>;
    fn list_properties(&self) -> Vec<::ParamSpec>;

    fn block_signal(&self, handler_id: &SignalHandlerId);
    fn unblock_signal(&self, handler_id: &SignalHandlerId);
    fn stop_signal_emission(&self, signal_name: &str);

    fn connect<'a, N, F>(&self, signal_name: N, after: bool, callback: F) -> Result<SignalHandlerId, BoolError>
        where N: Into<&'a str>, F: Fn(&[Value]) -> Option<Value> + Send + Sync + 'static;
    fn emit<'a, N: Into<&'a str>>(&self, signal_name: N, args: &[&ToValue]) -> Result<Option<Value>, BoolError>;
    fn disconnect(&self, handler_id: SignalHandlerId);

    fn connect_notify<'a, P: Into<Option<&'a str>>, F: Fn(&Self, &::ParamSpec) + Send + Sync + 'static>(&self, name: P, f: F) -> SignalHandlerId;
    fn notify<'a, N: Into<&'a str>>(&self, property_name: N);
    fn notify_by_pspec(&self, pspec: &::ParamSpec);

    fn downgrade(&self) -> WeakRef<Self>;

    fn bind_property<'a, O: IsA<Object>, N: Into<&'a str>, M: Into<&'a str>>(&'a self, source_property: N, target: &'a O, target_property: M) -> BindingBuilder<'a, Self, O>;

    fn ref_count(&self) -> u32;
}

impl<T: IsA<Object> + SetValue> ObjectExt for T {
    fn get_type(&self) -> Type {
        self.get_object_class().get_type()
    }

    fn get_object_class(&self) -> &ObjectClass {
        unsafe {
            let obj = self.to_glib_none().0;
            let klass = (*obj).g_type_instance.g_class as *const ObjectClass;
            &*klass
        }
    }

    fn set_property<'a, N: Into<&'a str>>(&self, property_name: N, value: &ToValue) -> Result<(), BoolError> {
        let property_name = property_name.into();
        let property_value = value.to_value();

        let pspec = match self.find_property(property_name) {
            Some(pspec) => pspec,
            None => {
                return Err(BoolError("property not found"));
            }
        };

        if !pspec.get_flags().contains(::ParamFlags::WRITABLE) || pspec.get_flags().contains(::ParamFlags::CONSTRUCT_ONLY) {
            return Err(BoolError("property is not writable"));
        }

        unsafe {
            // While GLib actually allows all types that can somehow be transformed
            // into the property type, we're more restrictive here to be consistent
            // with Rust's type rules. We only allow the exact same type, or if the
            // value type is a subtype of the property type
            let valid_type: bool = from_glib(gobject_ffi::g_type_check_value_holds(
                    mut_override(property_value.to_glib_none().0),
                    pspec.get_value_type().to_glib()));
            if !valid_type {
                return Err(BoolError("property can't be set from the given type"));
            }

            let changed: bool = from_glib(gobject_ffi::g_param_value_validate(
                    pspec.to_glib_none().0, mut_override(property_value.to_glib_none().0)));
            let change_allowed = pspec.get_flags().contains(::ParamFlags::LAX_VALIDATION);
            if changed && !change_allowed {
                return Err(BoolError("property can't be set from given value, it is invalid or out of range"));
            }

            gobject_ffi::g_object_set_property(self.to_glib_none().0,
                                               property_name.to_glib_none().0,
                                               property_value.to_glib_none().0);
        }

        Ok(())
    }

    fn get_property<'a, N: Into<&'a str>>(&self, property_name: N) -> Result<Value, BoolError> {
        let property_name = property_name.into();

        let pspec = match self.find_property(property_name) {
            Some(pspec) => pspec,
            None => {
                return Err(BoolError("property not found"));
            }
        };

        if !pspec.get_flags().contains(::ParamFlags::READABLE) {
            return Err(BoolError("property is not readable"));
        }

        unsafe {
            let mut value = Value::from_type(pspec.get_value_type());
            gobject_ffi::g_object_get_property(self.to_glib_none().0, property_name.to_glib_none().0, value.to_glib_none_mut().0);

            // This can't really happen unless something goes wrong inside GObject
            if value.type_() == ::Type::Invalid {
                Err(BoolError("Failed to get property value"))
            } else {
                Ok(value)
            }
        }
    }

    fn block_signal(&self, handler_id: &SignalHandlerId) {
        unsafe {
            gobject_ffi::g_signal_handler_block(self.to_glib_none().0, handler_id.to_glib());
        }
    }

    fn unblock_signal(&self, handler_id: &SignalHandlerId) {
        unsafe {
            gobject_ffi::g_signal_handler_unblock(self.to_glib_none().0, handler_id.to_glib());
        }
    }

    fn stop_signal_emission(&self, signal_name: &str) {
        unsafe {
            gobject_ffi::g_signal_stop_emission_by_name(self.to_glib_none().0, signal_name.to_glib_none().0);
        }
    }

    fn disconnect(&self, handler_id: SignalHandlerId) {
        unsafe {
            gobject_ffi::g_signal_handler_disconnect(self.to_glib_none().0, handler_id.to_glib());
        }
    }

    fn connect_notify<'a, P: Into<Option<&'a str>>, F: Fn(&Self, &::ParamSpec) + Send + Sync + 'static>(&self, name: P, f: F) -> SignalHandlerId {
        use std::mem::transmute;

        unsafe extern "C" fn notify_trampoline<P>(this: *mut gobject_ffi::GObject, param_spec: *mut gobject_ffi::GParamSpec, f: glib_ffi::gpointer)
        where P: IsA<Object> {
            let f: &&(Fn(&P, &::ParamSpec) + Send + Sync + 'static) = transmute(f);
            f(&Object::from_glib_borrow(this).downcast_unchecked(), &from_glib_borrow(param_spec))
        }

        let name = name.into();
        let signal_name = if let Some(name) = name {
            format!("notify::{}", name)
        } else {
            "notify".into()
        };

        unsafe {
            let f: Box<Box<Fn(&Self, &::ParamSpec) + Send + Sync + 'static>> = Box::new(Box::new(f));
            ::signal::connect(self.to_glib_none().0, &signal_name,
                transmute(notify_trampoline::<Self> as usize), Box::into_raw(f) as *mut _)
        }
    }

    fn notify<'a, N: Into<&'a str>>(&self, property_name: N) {
        let property_name = property_name.into();

        unsafe {
            gobject_ffi::g_object_notify(self.to_glib_none().0, property_name.to_glib_none().0);
        }
    }

    fn notify_by_pspec(&self, pspec: &::ParamSpec) {
        unsafe {
            gobject_ffi::g_object_notify_by_pspec(self.to_glib_none().0, pspec.to_glib_none().0);
        }
    }

    fn has_property<'a, N: Into<&'a str>>(&self, property_name: N, type_: Option<Type>) -> Result<(), BoolError> {
        self.get_object_class().has_property(property_name, type_)
    }

    fn get_property_type<'a, N: Into<&'a str>>(&self, property_name: N) -> Option<Type> {
        self.get_object_class().get_property_type(property_name)
    }

    fn find_property<'a, N: Into<&'a str>>(&self, property_name: N) -> Option<::ParamSpec> {
        self.get_object_class().find_property(property_name)
    }

    fn list_properties(&self) -> Vec<::ParamSpec> {
        self.get_object_class().list_properties()
    }

    fn connect<'a, N, F>(&self, signal_name: N, after: bool, callback: F) -> Result<SignalHandlerId, BoolError>
        where N: Into<&'a str>, F: Fn(&[Value]) -> Option<Value> + Send + Sync + 'static {
        let signal_name: &str = signal_name.into();

        unsafe {
            let type_ = self.get_type();

            let mut signal_id = 0;
            let mut signal_detail = 0;

            let found: bool = from_glib(gobject_ffi::g_signal_parse_name(signal_name.to_glib_none().0,
                                                                         type_.to_glib(), &mut signal_id,
                                                                         &mut signal_detail, true.to_glib()));

            if !found {
                return Err(BoolError("Signal not found"));
            }

            let mut details = mem::zeroed();
            gobject_ffi::g_signal_query(signal_id, &mut details);
            if details.signal_id != signal_id {
                return Err(BoolError("Signal not found"));
            }

            // This is actually G_SIGNAL_TYPE_STATIC_SCOPE
            let return_type: Type = from_glib(details.return_type & (!gobject_ffi::G_TYPE_FLAG_RESERVED_ID_BIT));
            let closure = Closure::new(move |values| {
                let ret = callback(values);

                if return_type == Type::Unit {
                    if let Some(ret) = ret {
                        panic!("Signal required no return value but got value of type {}", ret.type_().name());
                    }
                    None
                } else {
                    match ret {
                        Some(ret) => {
                            let valid_type: bool = from_glib(gobject_ffi::g_type_check_value_holds(
                                    mut_override(ret.to_glib_none().0),
                                    return_type.to_glib()));
                            if !valid_type {
                                panic!("Signal required return value of type {} but got {}",
                                       return_type.name(), ret.type_().name());
                            }
                            Some(ret)
                        },
                        None => {
                            panic!("Signal required return value of type {} but got None", return_type.name());
                        },
                    }
                }
            });
            let handler = gobject_ffi::g_signal_connect_closure_by_id(self.to_glib_none().0, signal_id, signal_detail,
                                                                      closure.to_glib_none().0, after.to_glib());

            if handler == 0 {
                Err(BoolError("Failed to connect to signal"))
            } else {
                Ok(from_glib(handler))
            }
        }
    }

    fn emit<'a, N: Into<&'a str>>(&self, signal_name: N, args: &[&ToValue]) -> Result<Option<Value>, BoolError> {
        let signal_name: &str = signal_name.into();
        unsafe {
            let type_ = self.get_type();

            let mut signal_id = 0;
            let mut signal_detail = 0;

            let found: bool = from_glib(gobject_ffi::g_signal_parse_name(signal_name.to_glib_none().0,
                                                                         type_.to_glib(), &mut signal_id,
                                                                         &mut signal_detail, true.to_glib()));

            if !found {
                return Err(BoolError("Signal not found"));
            }

            let mut details = mem::zeroed();
            gobject_ffi::g_signal_query(signal_id, &mut details);
            if details.signal_id != signal_id {
                return Err(BoolError("Signal not found"));
            }

            if details.n_params != args.len() as u32 {
                return Err(BoolError("Incompatible number of arguments"));
            }

            for i in 0..(details.n_params as usize) {
                let arg_type = *(details.param_types.add(i)) & (!gobject_ffi::G_TYPE_FLAG_RESERVED_ID_BIT);
                if arg_type != args[i].to_value_type().to_glib() {
                    return Err(BoolError("Incompatible argument types"));
                }
            }

            let mut v_args: Vec<Value>;
            let mut s_args: [Value; 10] = mem::zeroed();
            let args = if args.len() < 10 {
                for (i, arg) in iter::once(&(self as &ToValue)).chain(args).enumerate() {
                    s_args[i] = arg.to_value();
                }
                &s_args[0..args.len()+1]
            } else {
                v_args = Vec::with_capacity(args.len() + 1);
                for arg in iter::once(&(self as &ToValue)).chain(args) {
                    v_args.push(arg.to_value());
                }
                v_args.as_slice()
            };

            let mut return_value = Value::uninitialized();
            if details.return_type != gobject_ffi::G_TYPE_NONE {
                gobject_ffi::g_value_init(return_value.to_glib_none_mut().0, details.return_type);
            }

            gobject_ffi::g_signal_emitv(mut_override(args.as_ptr()) as *mut gobject_ffi::GValue,
                signal_id, signal_detail, return_value.to_glib_none_mut().0);

            if return_value.type_() != Type::Unit && return_value.type_() != Type::Invalid {
                Ok(Some(return_value))
            } else {
                Ok(None)
            }
        }
    }

    fn downgrade(&self) -> WeakRef<T> {
        unsafe {
            let w = WeakRef(Box::new(mem::uninitialized()), PhantomData);
            gobject_ffi::g_weak_ref_init(mut_override(&*w.0), self.to_glib_none().0);
            w
        }
    }

    fn bind_property<'a, O: IsA<Object>, N: Into<&'a str>, M: Into<&'a str>>(&'a self, source_property: N, target: &'a O, target_property: M) -> BindingBuilder<'a, Self, O> {
        let source_property = source_property.into();
        let target_property = target_property.into();

        BindingBuilder::new(self, source_property, target, target_property)
    }

    fn ref_count(&self) -> u32 {
        let stash = self.to_glib_none();
        let ptr: *mut gobject_ffi::GObject = stash.0;

        unsafe { glib_ffi::g_atomic_int_get(&(*ptr).ref_count as *const u32 as *const i32) as u32 }
    }
}

/// Class struct for `glib::Object`.
///
/// All actual functionality is provided via the [`ObjectClassExt`] trait.
///
/// [`ObjectClassExt`]: trait.ObjectClassExt.html
#[repr(C)]
pub struct ObjectClass(gobject_ffi::GObjectClass);

impl ObjectClass {
    pub fn has_property<'a, N: Into<&'a str>>(&self, property_name: N, type_: Option<Type>) -> Result<(), BoolError> {
        let property_name = property_name.into();
        let ptype = self.get_property_type(property_name);

        match (ptype, type_) {
            (None, _) => Err(BoolError("Invalid property name")),
            (Some(_), None) => Ok(()),
            (Some(ptype), Some(type_)) => {
                if ptype == type_ {
                    Ok(())
                } else {
                    Err(BoolError("Invalid property type"))
                }
            },
        }
    }

    pub fn get_property_type<'a, N: Into<&'a str>>(&self, property_name: N) -> Option<Type> {
        self.find_property(property_name).map(|pspec| pspec.get_value_type())
    }

    pub fn find_property<'a, N: Into<&'a str>>(&self, property_name: N) -> Option<::ParamSpec> {
        let property_name = property_name.into();
        unsafe {
            let klass = self as *const _ as *const gobject_ffi::GObjectClass;

            from_glib_none(gobject_ffi::g_object_class_find_property(klass as *mut _, property_name.to_glib_none().0))
        }
    }

    pub fn list_properties(&self) -> Vec<::ParamSpec> {
        unsafe {
            let klass = self as *const _ as *const gobject_ffi::GObjectClass;

            let mut n_properties = 0;

            let props = gobject_ffi::g_object_class_list_properties(klass as *mut _, &mut n_properties);
            FromGlibContainer::from_glib_none_num(props, n_properties as usize)
        }
    }
}

unsafe impl IsClassFor for ObjectClass {
    type Instance = Object;
}

unsafe impl Send for ObjectClass {}
unsafe impl Sync for ObjectClass {}

pub struct WeakRef<T: IsA<Object> + ?Sized>(Box<gobject_ffi::GWeakRef>, PhantomData<*const T>);

impl<T: IsA<Object> + StaticType + UnsafeFrom<ObjectRef> + Wrapper + ?Sized> WeakRef<T> {
    pub fn new() -> WeakRef<T> {
        unsafe {
            let w = WeakRef(Box::new(mem::uninitialized()), PhantomData);
            gobject_ffi::g_weak_ref_init(mut_override(&*w.0), ptr::null_mut());
            w
        }
    }

    pub fn upgrade(&self) -> Option<T> {
        unsafe {
            let ptr = gobject_ffi::g_weak_ref_get(mut_override(&*self.0));
            if ptr.is_null() {
                None
            } else {
                let obj: Object = from_glib_full(ptr);
                Some(T::from(obj.into()))
            }
        }
    }
}

impl<T: IsA<Object> + ?Sized> Drop for WeakRef<T> {
    fn drop(&mut self) {
        unsafe {
            gobject_ffi::g_weak_ref_clear(mut_override(&*self.0));
        }
    }
}

impl<T: IsA<Object> + ?Sized> Clone for WeakRef<T> {
    fn clone(&self) -> Self {
        unsafe {
            let c = WeakRef(Box::new(mem::uninitialized()), PhantomData);

            let o = gobject_ffi::g_weak_ref_get(mut_override(&*self.0));
            gobject_ffi::g_weak_ref_init(mut_override(&*c.0), o);
            if !o.is_null() {
                gobject_ffi::g_object_unref(o);
            }

            c
        }
    }
}

impl<T: IsA<Object>> Default for WeakRef<T> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<T: IsA<Object> + Sync + Sync> Sync for WeakRef<T> {}
unsafe impl<T: IsA<Object> + Send + Sync> Send for WeakRef<T> {}

/// A weak reference to the object it was created for that can be sent to
/// different threads even for object types that don't implement `Send`.
///
/// Trying to upgrade the weak reference from another thread than the one
/// where it was created on will panic but dropping or cloning can be done
/// safely from any thread.
pub struct SendWeakRef<T: IsA<Object>>(WeakRef<T>, Option<usize>);

impl<T: IsA<Object>> SendWeakRef<T> {
    pub fn new() -> SendWeakRef<T> {
        SendWeakRef(WeakRef::new(), None)
    }

    pub fn into_weak_ref(self) -> WeakRef<T> {
        if self.1.is_some() && self.1 != Some(get_thread_id()) {
            panic!("SendWeakRef dereferenced on a different thread");
        }

        self.0
    }
}

impl<T: IsA<Object>> ops::Deref for SendWeakRef<T> {
    type Target = WeakRef<T>;

    fn deref(&self) -> &WeakRef<T> {
        if self.1.is_some() && self.1 != Some(get_thread_id()) {
            panic!("SendWeakRef dereferenced on a different thread");
        }

        &self.0
    }
}

// Deriving this gives the wrong trait bounds
impl<T: IsA<Object>> Clone for SendWeakRef<T> {
    fn clone(&self) -> Self {
        SendWeakRef(self.0.clone(), self.1.clone())
    }
}

impl<T: IsA<Object>> Default for SendWeakRef<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: IsA<Object>> From<WeakRef<T>> for SendWeakRef<T> {
    fn from(v: WeakRef<T>) -> SendWeakRef<T> {
        SendWeakRef(v, Some(get_thread_id()))
    }
}

unsafe impl<T: IsA<Object>> Sync for SendWeakRef<T> {}
unsafe impl<T: IsA<Object>> Send for SendWeakRef<T> {}

pub struct BindingBuilder<'a, S: IsA<Object> + 'a, T: IsA<Object> + 'a> {
    source: &'a S,
    source_property: &'a str,
    target: &'a T,
    target_property: &'a str,
    flags: ::BindingFlags,
    transform_to: Option<::Closure>,
    transform_from: Option<::Closure>,
}

impl<'a, S: IsA<Object> + 'a, T: IsA<Object> + 'a> BindingBuilder<'a, S, T> {
    fn new(source: &'a S, source_property: &'a str, target: &'a T, target_property: &'a str) -> Self {
        Self { source, source_property, target, target_property, flags: ::BindingFlags::DEFAULT, transform_to: None, transform_from: None }
    }

    fn transform_closure<F: Fn(&::Binding, &Value) -> Option<Value> + Send + Sync + 'static>(func: F) -> ::Closure {
        ::Closure::new(move |values| {
            assert_eq!(values.len(), 3);
            let binding = values[0].get::<::Binding>().unwrap();
            let from = unsafe {
                let ptr = gobject_ffi::g_value_get_boxed(mut_override(&values[1] as *const Value as *const gobject_ffi::GValue));
                assert!(!ptr.is_null());
                &*(ptr as *const gobject_ffi::GValue as *const Value)
            };

            match func(&binding, &from) {
                None => Some(false.to_value()),
                Some(value) => {
                    unsafe {
                        gobject_ffi::g_value_set_boxed(mut_override(&values[2] as *const Value as *const gobject_ffi::GValue), &value as *const Value as *const _);
                    }

                    Some(true.to_value())
                }
            }
        })
    }

    pub fn transform_from<F: Fn(&::Binding, &Value) -> Option<Value> + Send + Sync + 'static>(self, func: F) -> Self {
        Self {
            transform_from: Some(Self::transform_closure(func)),
            ..self
        }
    }

    pub fn transform_to<F: Fn(&::Binding, &Value) -> Option<Value> + Send + Sync + 'static>(self, func: F) -> Self {
        Self {
            transform_to: Some(Self::transform_closure(func)),
            ..self
        }
    }

    pub fn flags(self, flags: ::BindingFlags) -> Self {
        Self {
            flags: flags,
            ..self
        }
    }

    pub fn build(self) -> Option<::Binding> {
        unsafe {
            from_glib_none(
                gobject_ffi::g_object_bind_property_with_closures(
                    self.source.to_glib_none().0,
                    self.source_property.to_glib_none().0,
                    self.target.to_glib_none().0,
                    self.target_property.to_glib_none().0,
                    self.flags.to_glib(),
                    self.transform_to.to_glib_none().0,
                    self.transform_from.to_glib_none().0,
                )
            )
        }
    }
}
