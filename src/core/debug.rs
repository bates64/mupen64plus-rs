use crate::Error;
use super::{Core, Mupen};
use mupen64plus_sys::*;
use std::sync::Mutex;
use std::rc::Rc;
use std::ops::{RangeBounds, Bound};

// thread_local is OK because Debugger is !Send (due to Rc)
thread_local! {
    static INIT_SUBSCRIBERS: Mutex<Vec<Box<dyn FnMut()>>> = Mutex::new(Vec::new());
    static UPDATE_SUBSCRIBERS: Mutex<Vec<Box<dyn FnMut(u32)>>> = Mutex::new(Vec::new());
    static VI_SUBSCRIBERS: Mutex<Vec<Box<dyn FnMut()>>> = Mutex::new(Vec::new());
}

pub(super) fn clear_subscribers() {
    INIT_SUBSCRIBERS.with(|s| s.lock().unwrap().clear());
    UPDATE_SUBSCRIBERS.with(|s| s.lock().unwrap().clear());
    VI_SUBSCRIBERS.with(|s| s.lock().unwrap().clear());
}

extern "C" fn callback_init() {
    INIT_SUBSCRIBERS.with(|subscribers| {
        let mut subscribers = subscribers.lock().unwrap();
        for subscriber in subscribers.iter_mut() {
            subscriber();
        }
    });
}

extern "C" fn callback_update(pc: u32) {
    UPDATE_SUBSCRIBERS.with(|subscribers| {
        let mut subscribers = subscribers.lock().unwrap();
        for subscriber in subscribers.iter_mut() {
            subscriber(pc);
        }
    });
}

extern "C" fn callback_vi() {
    VI_SUBSCRIBERS.with(|subscribers| {
        let mut subscribers = subscribers.lock().unwrap();
        for subscriber in subscribers.iter_mut() {
            subscriber();
        }
    });
}

/// Handle to debugger API. Uses reference-counting for cheap cloning (e.g. passing to closures).
#[derive(Clone)]
pub struct Debugger {
    core: Rc<Core>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    /// Continue execution until a breakpoint is hit or a different state is chosen.
    Running,
    /// Pause execution to allow manual stepping.
    Paused,
    /// Sends callbacks as each step is performed.
    Stepping,
}

/// CPU registers. Writing is allowed.
#[repr(C)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Registers {
    pub r0: u64,
    pub at: u64,
    pub v0: u64,
    pub v1: u64,
    pub a0: u64,
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub t3: u64,
    pub t4: u64,
    pub t5: u64,
    pub t6: u64,
    pub t7: u64,
    pub s0: u64,
    pub s1: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub t8: u64,
    pub t9: u64,
    pub k0: u64,
    pub k1: u64,
    pub gp: u64,
    pub sp: u64,
    pub fp: u64,
    pub ra: u64,
}

pub struct Breakpoint(m64p_breakpoint);

pub fn breakpoint<R: RangeBounds<u32>>(address: R, exec: bool, read: bool, write: bool, log: bool) -> Breakpoint {
    Breakpoint(m64p_breakpoint {
        address: match address.start_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => n + 1,
            Bound::Unbounded => 0,
        },
        endaddr: match address.end_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => n - 1,
            Bound::Unbounded => u32::MAX,
        },
        flags: {
            let mut flags = m64p_dbg_bkp_flags_M64P_BKP_FLAG_ENABLED;
            if exec {
                flags |= m64p_dbg_bkp_flags_M64P_BKP_FLAG_EXEC;
            }
            if read {
                flags |= m64p_dbg_bkp_flags_M64P_BKP_FLAG_READ;
            }
            if write {
                flags |= m64p_dbg_bkp_flags_M64P_BKP_FLAG_WRITE;
            }
            if log {
                flags |= m64p_dbg_bkp_flags_M64P_BKP_FLAG_LOG;
            }
            flags
        }
    })
}

impl Breakpoint {
    pub fn new(address: u32) -> Breakpoint {
        breakpoint(address..=address, false, false, false, false)
    }

    pub fn range<B: RangeBounds<u32>>(range: B) -> Breakpoint {
        breakpoint(range, false, false, false, false)
    }

    pub fn exec(mut self) -> Self {
        self.0.flags |= m64p_dbg_bkp_flags_M64P_BKP_FLAG_EXEC;
        self
    }

    pub fn read(mut self) -> Self {
        self.0.flags |= m64p_dbg_bkp_flags_M64P_BKP_FLAG_READ;
        self
    }

    pub fn write(mut self) -> Self {
        self.0.flags |= m64p_dbg_bkp_flags_M64P_BKP_FLAG_WRITE;
        self
    }

    pub fn log(mut self) -> Self {
        self.0.flags |= m64p_dbg_bkp_flags_M64P_BKP_FLAG_LOG;
        self
    }

    pub fn disable(mut self) -> Self {
        self.0.flags &= m64p_dbg_bkp_flags_M64P_BKP_FLAG_ENABLED;
        self
    }
}


impl From<u32> for Breakpoint {
    fn from(address: u32) -> Breakpoint {
        Breakpoint(m64p_breakpoint {
            address,
            endaddr: address,
            flags: m64p_dbg_bkp_flags_M64P_BKP_FLAG_ENABLED
                | m64p_dbg_bkp_flags_M64P_BKP_FLAG_EXEC,
        })
    }
}

impl Mupen {
    /// Set up and enable the debugger. Note that calling this will cause the
    /// emulator to immediately pause at the first instruction; in an on_init
    /// callback, you can call `debugger.run()` to resume execution.
    ///
    /// Returns `Error::Unsupported` if the core was not compiled with the debugger enabled (`DEBUGGER=1`).
    pub fn debug(&self) -> Result<Debugger, Error> {
        use std::ffi::CStr;

        if self.is_debug_supported() {
            // Setup init/update/vi callbacks
            let ret = unsafe {
                self.core.debug_set_callbacks.unwrap()(
                    Some(callback_init),
                    Some(callback_update),
                    Some(callback_vi),
                )
            };
            if ret != m64p_error_M64ERR_SUCCESS {
                return Err(ret.into());
            }

            // Enable debugger in core config
            unsafe {
                let mut core_config = std::ptr::null_mut();

                let ret = self.core.config_open_section.unwrap()(
                    CStr::from_bytes_with_nul_unchecked(b"Core\0").as_ptr(),
                    &mut core_config,       
                );
                if ret != m64p_error_M64ERR_SUCCESS {
                    return Err(ret.into());
                }

                assert!(!core_config.is_null());

                // EnableDebugger = 1
                let ret = self.core.config_set_parameter.unwrap()(
                    core_config,
                    CStr::from_bytes_with_nul_unchecked(b"EnableDebugger\0").as_ptr(),
                    m64p_type_M64TYPE_BOOL,
                    &mut 1 as *mut _ as *mut _,
                );
                if ret != m64p_error_M64ERR_SUCCESS {
                    return Err(ret.into());
                }

                // R4300Emulator = 0 (pure interpreter)
                let ret = self.core.config_set_parameter.unwrap()(
                    core_config,
                    CStr::from_bytes_with_nul_unchecked(b"R4300Emulator\0").as_ptr(),
                    m64p_type_M64TYPE_INT,
                    &mut 0 as *mut _ as *mut _,
                );
                if ret != m64p_error_M64ERR_SUCCESS {
                    return Err(ret.into());
                }
            }

            Ok(Debugger {
                core: self.core.clone(),
            })
        } else {
            Err(Error::Unsupported)
        }
    }

    /// Returns true if the core supports debugger functions.
    pub fn is_debug_supported(&self) -> bool {
        use crate::plugin::Capability;

        self.core.get_version()
            .and_then(|version| {
                if version.capabilities.contains(Capability::DEBUGGER) {
                    Ok(())
                } else {
                    Err(Error::Unsupported)
                }
            })
            .is_ok()
    }
}

#[allow(non_upper_case_globals)]
impl Debugger {
    fn get_state_prop(&self, prop: m64p_dbg_state) -> i32 {
        unsafe {
            self.core.debug_get_state.unwrap()(prop)
        }
    }

    pub fn run_state(&self) -> RunState {
        match self.get_state_prop(m64p_dbg_state_M64P_DBG_RUN_STATE) as u32 {
            m64p_dbg_runstate_M64P_DBG_RUNSTATE_RUNNING => RunState::Running,
            m64p_dbg_runstate_M64P_DBG_RUNSTATE_PAUSED => RunState::Paused,
            m64p_dbg_runstate_M64P_DBG_RUNSTATE_STEPPING => RunState::Stepping,
            n => panic!("invalid m64p_dbg_runstate: {}", n),
        }
    }

    pub fn set_run_state(&self, state: RunState) -> Result<(), Error> {
        let ret = unsafe {
            self.core.debug_set_run_state.unwrap()(match state {
                RunState::Running => m64p_dbg_runstate_M64P_DBG_RUNSTATE_RUNNING,
                RunState::Paused => m64p_dbg_runstate_M64P_DBG_RUNSTATE_PAUSED,
                RunState::Stepping => m64p_dbg_runstate_M64P_DBG_RUNSTATE_STEPPING,
            })
        };

        if ret == m64p_error_M64ERR_SUCCESS {
            Ok(())
        } else {
            Err(ret.into())
        }
    }

    pub fn run(&self) -> Result<(), Error> {
        self.set_run_state(RunState::Running)
    }

    pub fn pause(&self) -> Result<(), Error> {
        self.set_run_state(RunState::Paused)
    }

    pub fn step(&self) -> Result<(), Error> {
        self.set_run_state(RunState::Stepping)?;

        let ret = unsafe {
            self.core.debug_step.unwrap()()
        };

        if ret == m64p_error_M64ERR_SUCCESS {
            Ok(())
        } else {
            Err(ret.into())
        }
    }

    /// Provide a callback for start-of-execution.
    pub fn on_init(&self, callback: Box<dyn FnMut()>) {
        INIT_SUBSCRIBERS.with(|s| s.lock().unwrap().push(callback));
    }

    /// Provide a callback for steps/breakpoints.
    pub fn on_update(&self, callback: Box<dyn FnMut(u32)>) {
        UPDATE_SUBSCRIBERS.with(|s| s.lock().unwrap().push(callback));
    }

    /// Provide a callback for vertical interrupts.
    pub fn on_vi(&self, callback: Box<dyn FnMut()>) {
        VI_SUBSCRIBERS.with(|s| s.lock().unwrap().push(callback));
    }

    /// Get the value of the PC register (address of next instruction).
    pub fn pc(&self) -> u32 {
        unsafe {
            let pc = self.core.debug_get_cpu_data_ptr.unwrap()(m64p_dbg_cpu_data_M64P_CPU_PC) as *mut u32;
            *pc
        }
    }

    /// Get the previous PC register (address of the instruction we just executed).
    pub fn prev_pc(&self) -> u32 {
        unsafe {
            self.core.debug_get_state.unwrap()(m64p_dbg_state_M64P_DBG_PREVIOUS_PC) as u32
        }
    }

    /// Get access to the CPU registers.
    pub fn registers(&self) -> &mut Registers {
        unsafe {
            let regs = self.core.debug_get_cpu_data_ptr.unwrap()(m64p_dbg_cpu_data_M64P_CPU_REG_REG);
            std::mem::transmute(regs)
        }
    }

    pub fn disassemble(&self, instruction: u32, pc: u32) -> (String, String) {
        use std::ffi::CString;

        let mnemonic = CString::new("        ").unwrap().into_raw();
        let args = CString::new("                ").unwrap().into_raw();

        unsafe {
            self.core.debug_decode_op.unwrap()(
                instruction,
                mnemonic,
                args,
                pc as i32,
            );

            (
                CString::from_raw(mnemonic).into_string().unwrap(),
                CString::from_raw(args).into_string().unwrap(),
            )
        }
    }

    pub fn add_breakpoint<B: Into<Breakpoint>>(&self, bp: B) -> u32 {
        let mut bp = bp.into();

        let index = unsafe {
            self.core.debug_breakpoint_command.unwrap()(
                m64p_dbg_bkp_command_M64P_BKP_CMD_ADD_STRUCT,
                0,
                &mut bp.0 as *mut _,
            )
        };
        assert!(index != -1);
        index as u32
    }

    pub fn replace_breakpoint<B: Into<Breakpoint>>(&self, idx: u32, bp: B) {
        let mut bp = bp.into();

        unsafe {
            self.core.debug_breakpoint_command.unwrap()(
                m64p_dbg_bkp_command_M64P_BKP_CMD_REPLACE,
                idx,
                &mut bp.0 as *mut _,
            );
        }
    }

    pub fn remove_breakpoint(&self, idx: u32) {
        unsafe {
            self.core.debug_breakpoint_command.unwrap()(
                m64p_dbg_bkp_command_M64P_BKP_CMD_REMOVE_IDX,
                idx,
                std::ptr::null_mut(),
            );
        }
    }

    pub fn remove_breakpoint_by_address(&self, address: u32) {
        unsafe {
            self.core.debug_breakpoint_command.unwrap()(
                m64p_dbg_bkp_command_M64P_BKP_CMD_REMOVE_ADDR,
                address,
                std::ptr::null_mut(),
            );
        }
    }

    pub fn enable_breakpoint(&self, idx: u32) {
        unsafe {
            self.core.debug_breakpoint_command.unwrap()(
                m64p_dbg_bkp_command_M64P_BKP_CMD_ENABLE,
                idx,
                std::ptr::null_mut(),
            );
        }
    }

    pub fn disable_breakpoint(&self, idx: u32) {
        unsafe {
            self.core.debug_breakpoint_command.unwrap()(
                m64p_dbg_bkp_command_M64P_BKP_CMD_DISABLE,
                idx,
                std::ptr::null_mut(),
            );
        }
    }

    pub fn find_exec_breakpoint(&self, address: u32) -> Option<u32> {
        let r = unsafe {
            self.core.debug_breakpoint_command.unwrap()(
                m64p_dbg_bkp_command_M64P_BKP_CMD_CHECK,
                address,
                std::ptr::null_mut(),
            )
        };

        if r == -1 {
            None
        } else {
            Some(r as u32)
        }
    }

    pub fn read_u64(&self, address: u32) -> u64 {
        unsafe {
            self.core.debug_mem_read64.unwrap()(address)
        }
    }

    pub fn write_u64(&self, address: u32, value: u64) {
        unsafe {
            self.core.debug_mem_write64.unwrap()(address, value)
        }
    }

    pub fn read_u32(&self, address: u32) -> u32 {
        unsafe {
            self.core.debug_mem_read32.unwrap()(address)
        }
    }

    pub fn write_u32(&self, address: u32, value: u32) {
        unsafe {
            self.core.debug_mem_write32.unwrap()(address, value)
        }
    }

    pub fn read_u16(&self, address: u32) -> u16 {
        unsafe {
            self.core.debug_mem_read16.unwrap()(address)
        }
    }

    pub fn write_u16(&self, address: u32, value: u16) {
        unsafe {
            self.core.debug_mem_write16.unwrap()(address, value)
        }
    }

    pub fn read_u8(&self, address: u32) -> u8 {
        unsafe {
            self.core.debug_mem_read8.unwrap()(address)
        }
    }

    pub fn write_u8(&self, address: u32, value: u8) {
        unsafe {
            self.core.debug_mem_write8.unwrap()(address, value)
        }
    }
}
