#[doc = r" The RTIC application module"] pub mod app
{
    #[doc =
    r" Always include the device crate which contains the vector table"] use
    pac as you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml ; use
    super :: * ; #[doc = r" User code from within the module"]
    #[doc = r" User code end"] #[doc = " User provided init function"]
    #[inline(always)] #[allow(non_snake_case)] fn init(cx : init :: Context)
    -> (Shared, Local, init :: Monotonics)
    { (Shared { a : 1 }, Local { b : 2 }, init :: Monotonics()) }
    #[doc = " User provided idle function"] #[allow(non_snake_case)] fn
    idle(cx : idle :: Context) ->!
    {
        use rtic :: Mutex as _ ; use rtic :: mutex :: prelude :: * ; let b =
        cx.local.b ; loop { asm :: nop() ; }
    } #[doc = " RTIC shared resource struct"] struct Shared { a : u16, }
    #[doc = " RTIC local resource struct"] struct Local { b : u16, }
    #[doc = r" Monotonics used by the system"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_Monotonics() ;
    #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_init_Context <
    'a >
    {
        #[doc = r" Core (Cortex-M) peripherals"] pub core : rtic :: export ::
        Peripherals, #[doc = r" Critical section token for init"] pub cs :
        rtic :: export :: CriticalSection < 'a >,
    } impl < 'a > __rtic_internal_init_Context < 'a >
    {
        #[doc(hidden)] #[inline(always)] pub unsafe fn
        new(core : rtic :: export :: Peripherals,) -> Self
        {
            __rtic_internal_init_Context
            { cs : rtic :: export :: CriticalSection :: new(), core, }
        }
    } #[allow(non_snake_case)] #[doc = " Initialization function"] pub mod
    init
    {
        #[doc(inline)] pub use super :: __rtic_internal_Monotonics as
        Monotonics ; #[doc(inline)] pub use super ::
        __rtic_internal_init_Context as Context ;
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = " Local resources `idle` has access to"] pub struct
    __rtic_internal_idleLocalResources < >
    { #[doc = " Local resource `b`"] pub b : & 'static mut u16, }
    #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_idle_Context < >
    {
        #[doc = r" Local Resources this task has access to"] pub local : idle
        :: LocalResources < >,
    } impl < > __rtic_internal_idle_Context < >
    {
        #[doc(hidden)] #[inline(always)] pub unsafe fn
        new(priority : & rtic :: export :: Priority) -> Self
        {
            __rtic_internal_idle_Context
            { local : idle :: LocalResources :: new(), }
        }
    } #[allow(non_snake_case)] #[doc = " Idle loop"] pub mod idle
    {
        #[doc(inline)] pub use super :: __rtic_internal_idleLocalResources as
        LocalResources ; #[doc(inline)] pub use super ::
        __rtic_internal_idle_Context as Context ;
    } mod shared_resources
    {
        use rtic :: export :: Priority ; #[doc(hidden)]
        #[allow(non_camel_case_types)] pub struct a_that_needs_to_be_locked <
        'a > { priority : & 'a Priority, } impl < 'a >
        a_that_needs_to_be_locked < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { a_that_needs_to_be_locked { priority } } #[inline(always)]
            pub unsafe fn priority(& self) -> & Priority { self.priority }
        }
    } #[doc = r" App module"] impl < > __rtic_internal_idleLocalResources < >
    {
        #[inline(always)] #[doc(hidden)] pub unsafe fn new() -> Self
        {
            __rtic_internal_idleLocalResources
            {
                b : & mut *
                (& mut *
                __rtic_internal_local_resource_b.get_mut()).as_mut_ptr(),
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic0"] static
    __rtic_internal_shared_resource_a : rtic :: RacyCell < core :: mem ::
    MaybeUninit < u16 >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic :: Mutex
    for shared_resources :: a_that_needs_to_be_locked < 'a >
    {
        type T = u16 ; #[inline(always)] fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl FnOnce(& mut u16) -> RTIC_INTERNAL_R) ->
        RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 0u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_a.get_mut() as * mut _,
                self.priority(), CEILING, pac :: NVIC_PRIO_BITS, &
                __rtic_internal_MASKS, f,)
            }
        }
    } #[doc(hidden)] #[allow(non_upper_case_globals)] const
    __rtic_internal_MASK_CHUNKS : usize = rtic :: export ::
    compute_mask_chunks([]) ; #[doc(hidden)] #[allow(non_upper_case_globals)]
    const __rtic_internal_MASKS :
    [rtic :: export :: Mask < __rtic_internal_MASK_CHUNKS > ; 3] =
    [rtic :: export :: create_mask([]), rtic :: export :: create_mask([]),
    rtic :: export :: create_mask([])] ; #[allow(non_camel_case_types)]
    #[allow(non_upper_case_globals)] #[doc(hidden)]
    #[link_section = ".uninit.rtic1"] static __rtic_internal_local_resource_b
    : rtic :: RacyCell < core :: mem :: MaybeUninit < u16 >> = rtic ::
    RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ; #[doc(hidden)]
    mod rtic_ext
    {
        use super :: * ; #[no_mangle] unsafe extern "C" fn main() ->!
        {
            const _CONST_CHECK : () =
            { if! rtic :: export :: have_basepri() {} else {} } ; let _ =
            _CONST_CHECK ; rtic :: export :: interrupt :: disable() ; let mut
            core : rtic :: export :: Peripherals = rtic :: export ::
            Peripherals :: steal().into() ; #[inline(never)] fn
            __rtic_init_resources < F > (f : F) where F : FnOnce() { f() ; }
            __rtic_init_resources(||
            {
                let(shared_resources, local_resources, mut monotonics) =
                init(init :: Context :: new(core.into())) ;
                __rtic_internal_local_resource_b.get_mut().write(core :: mem
                :: MaybeUninit :: new(local_resources.b)) ; rtic :: export ::
                interrupt :: enable() ;
            }) ;
            idle(idle :: Context ::
            new(& rtic :: export :: Priority :: new(0)))
        }
    }
}