//! linux-parity: partial
//! linux-source: vendor/linux/kernel/module/main.c
//! test-origin: linux:vendor/linux/tools/testing/selftests/module
//! Module constructor discovery and invocation.
//!
//! Linux discovers the table in `find_module_sections()` and invokes it from
//! `do_mod_ctors()` immediately before `mod->init`.  Relocations must already
//! have converted every table element into a runtime function pointer.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructorError {
    BothFormats,
    Misaligned,
    MalformedSize,
    NullEntry,
}

pub type ConstructorFn = unsafe extern "C" fn();

/// Relocated constructor table retained in module memory.
#[derive(Clone, Copy, Debug)]
pub struct ModuleConstructors {
    address: usize,
    count: usize,
}

impl ModuleConstructors {
    /// Select `.ctors`, falling back to `.init_array`, exactly as Linux does.
    pub fn discover(
        ctors: Option<(usize, usize)>,
        init_array: Option<(usize, usize)>,
    ) -> Result<Option<Self>, ConstructorError> {
        if ctors.is_some() && init_array.is_some() {
            return Err(ConstructorError::BothFormats);
        }
        let Some((address, bytes)) = ctors.or(init_array) else {
            return Ok(None);
        };
        if bytes % core::mem::size_of::<ConstructorFn>() != 0 {
            return Err(ConstructorError::MalformedSize);
        }
        if address % core::mem::align_of::<ConstructorFn>() != 0 {
            return Err(ConstructorError::Misaligned);
        }
        Ok(Some(Self {
            address,
            count: bytes / core::mem::size_of::<ConstructorFn>(),
        }))
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Invoke constructors in link order before the module init function.
    ///
    /// # Safety
    ///
    /// The table must remain mapped and executable targets must have passed
    /// the module loader's relocation and executable-range validation.
    pub unsafe fn run(&self) -> Result<(), ConstructorError> {
        let entries =
            unsafe { core::slice::from_raw_parts(self.address as *const usize, self.count) };
        for address in entries {
            if *address == 0 {
                return Err(ConstructorError::NullEntry);
            }
            let constructor: ConstructorFn = unsafe { core::mem::transmute(*address) };
            unsafe { constructor() };
        }
        Ok(())
    }
}
