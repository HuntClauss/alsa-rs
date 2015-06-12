//! HCtl API - for mixer control and jack detection
use alsa;
use std::ffi::{CStr};
use super::error::*;
use std::ptr;
use super::ctl_int;


/// [snd_hctl_t](http://www.alsa-project.org/alsa-doc/alsa-lib/group___h_control.html) wrapper
pub struct HCtl(*mut alsa::snd_hctl_t);

impl Drop for HCtl {
    fn drop(&mut self) { unsafe { alsa::snd_hctl_close(self.0) }; }
}

impl HCtl {
    /// Open does not support async mode (it's not very Rustic anyway)
    /// Note: You probably want to call `load` afterwards.
    pub fn open(c: &CStr, nonblock: bool) -> Result<HCtl> {
        let mut r = ptr::null_mut();
        let flags = if nonblock { 1 } else { 0 }; // FIXME: alsa::SND_CTL_NONBLOCK does not exist in alsa-sys
        check("snd_hctl_open", unsafe { alsa::snd_hctl_open(&mut r, c.as_ptr(), flags) })
            .map(|_| HCtl(r))
    }

    pub fn load(&self) -> Result<()> { check("snd_hctl_load", unsafe { alsa::snd_hctl_load(self.0) }).map(|_| ()) }

    pub fn elem_iter<'a>(&'a self) -> ElemIter<'a> { ElemIter(self, ptr::null_mut()) }
}

/// Iterates over elements for a `HCtl`
pub struct ElemIter<'a>(&'a HCtl, *mut alsa::snd_hctl_elem_t);

impl<'a> Iterator for ElemIter<'a> {
    type Item = Elem<'a>;
    fn next(&mut self) -> Option<Elem<'a>> {
        self.1 = if self.1 == ptr::null_mut() { unsafe { alsa::snd_hctl_first_elem((self.0).0) }}
            else { unsafe { alsa::snd_hctl_elem_next(self.1) }};
        if self.1 == ptr::null_mut() { None }
        else { Some(Elem(self.0, self.1)) }
    }
}


/// [snd_hctl_elem_t](http://www.alsa-project.org/alsa-doc/alsa-lib/group___h_control.html) wrapper
pub struct Elem<'a>(&'a HCtl, *mut alsa::snd_hctl_elem_t);

impl<'a> Elem<'a> {
    pub fn get_id(&self) -> Result<ctl_int::ElemId> {
        let v = try!(ctl_int::elem_id_new());
        unsafe { alsa::snd_hctl_elem_get_id(self.1, ctl_int::elem_id_ptr(&v)) };
        Ok(v)
    }
    pub fn info(&self) -> Result<ctl_int::ElemInfo> {
        let v = try!(ctl_int::elem_info_new());
        check("snd_hctl_elem_info", unsafe { alsa::snd_hctl_elem_info(self.1, ctl_int::elem_info_ptr(&v)) }).map(|_| v)
    }
    pub fn read(&self) -> Result<ctl_int::ElemValue> {
        let i = try!(self.info());
        let v = try!(ctl_int::elem_value_new(i.get_type(), i.get_count()));
        check("snd_hctl_elem_read", unsafe { alsa::snd_hctl_elem_read(self.1, ctl_int::elem_value_ptr(&v)) }).map(|_| v)
    }

    pub fn write(&self, v: &ctl_int::ElemValue) -> Result<bool> { check("snd_hctl_elem_write",
        unsafe { alsa::snd_hctl_elem_write(self.1, ctl_int::elem_value_ptr(&v)) }).map(|e| e > 0) }
}

#[test]
fn print_hctls() {
    for a in super::card::Iter::new().map(|x| x.unwrap()) {
        use std::ffi::CString;
        let h = HCtl::open(&CString::new(format!("hw:{}", a.get_index())).unwrap(), false).unwrap();
        h.load().unwrap();
        println!("Card {}:", a.get_name().unwrap());
        for b in h.elem_iter() {
            println!("  {:?} - {:?}", b.get_id().unwrap(), b.read().unwrap());
        }
    }
}
