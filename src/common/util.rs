#[allow(dead_code)]
#[inline]
fn to_binary<T: Sized>(thing: &T) -> &[u8] {
    let ptr = thing as *const T;
    unsafe { core::slice::from_raw_parts(ptr as *const u8, core::mem::size_of::<T>()) }
}

#[allow(dead_code)]
#[inline]
fn to_binary_mut<T: Sized>(thing: &mut T) -> &mut [u8] {
    let ptr = thing as *mut T;
    unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, core::mem::size_of::<T>()) }
}
