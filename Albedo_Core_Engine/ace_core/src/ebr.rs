// ============================================================================
// Albedo Core Engine (ACE)
// File: ebr.rs
// Description: Epoch-Based Reclamation (EBR) Completo.
//              Gerenciamento de memória lock-free seguro para multi-threading.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::sync::{Arc, RwLock, OnceLock};

/// A Época Global. Começa em 1.
static GLOBAL_EPOCH: AtomicU64 = AtomicU64::new(1);

/// Registro global de todas as threads participantes do EBR.
static THREAD_REGISTRY: OnceLock<RwLock<Vec<Arc<ThreadState>>>> = OnceLock::new();

fn registry() -> &'static RwLock<Vec<Arc<ThreadState>>> {
    THREAD_REGISTRY.get_or_init(|| RwLock::new(Vec::new()))
}

/// Estado de uma única thread participante.
pub struct ThreadState {
    epoch: AtomicU64,
    active: AtomicBool,
}

thread_local! {
    /// O estado local desta thread. Inicializado na primeira vez que `Guard::pin()` é chamado.
    static LOCAL_STATE: Arc<ThreadState> = {
        let state = Arc::new(ThreadState {
            epoch: AtomicU64::new(0),
            active: AtomicBool::new(false),
        });
        registry().write().unwrap().push(state.clone());
        state
    };

    /// Fila de ponteiros adiados: (ponteiro, função de drop, época_de_descarte).
    static DEFER_QUEUE: std::cell::RefCell<Vec<(*mut (), unsafe fn(*mut ()), u64)>> = std::cell::RefCell::new(Vec::new());
}

/// Um "Guardião" (Guard) sinaliza que a thread atual está ativamente lendo.
pub struct Guard {
    _private: (),
}

impl Guard {
    /// Registra a thread atual no gerenciador de épocas e previne coleta de lixo.
    #[inline]
    pub fn pin() -> Self {
        LOCAL_STATE.with(|state| {
            // Marca como ativa e sincroniza com a época global.
            state.active.store(true, Ordering::SeqCst);
            let global = GLOBAL_EPOCH.load(Ordering::SeqCst);
            state.epoch.store(global, Ordering::SeqCst);
        });
        Self { _private: () }
    }

    /// Tenta avançar a época global e coletar lixo se for seguro.
    pub fn flush(&self) {
        try_advance();
        collect_garbage();
    }
}

impl Drop for Guard {
    #[inline]
    fn drop(&mut self) {
        LOCAL_STATE.with(|state| {
            state.active.store(false, Ordering::SeqCst);
        });

        // Ocasionalmente, quando despinamos, tentamos limpar nosso lixo.
        // Isso evita acumular lixo indefinidamente se a thread for muito ativa.
        collect_garbage();
    }
}

/// Tenta avançar a `GLOBAL_EPOCH`.
/// Isso só tem sucesso se TODAS as threads ativas estiverem na época atual ou mais novas.
fn try_advance() {
    let global = GLOBAL_EPOCH.load(Ordering::SeqCst);
    let reg = registry().read().unwrap();

    for state in reg.iter() {
        if state.active.load(Ordering::SeqCst) {
            let thread_epoch = state.epoch.load(Ordering::SeqCst);
            if thread_epoch < global {
                // Alguma thread ativa ainda está presa no passado.
                return;
            }
        }
    }

    // Todas as threads ativas alcançaram a época global. Podemos avançar!
    // Usamos compare_exchange para evitar corrida se múltiplas threads tentarem avançar.
    let _ = GLOBAL_EPOCH.compare_exchange(global, global + 1, Ordering::SeqCst, Ordering::Relaxed);
}

/// Varre a `DEFER_QUEUE` local e descarta os objetos cuja época expirou.
fn collect_garbage() {
    let global = GLOBAL_EPOCH.load(Ordering::SeqCst);

    DEFER_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        let mut i = 0;
        while i < queue.len() {
            let (_, _, drop_epoch) = queue[i];

            // Regra do EBR: Se um objeto foi retirado na época `E`, ele é seguro
            // para deleção quando a Época Global atingir `E + 2`.
            if global >= drop_epoch + 2 {
                // Remove da fila e deleta
                let (ptr, dropper, _) = queue.remove(i);
                unsafe { dropper(ptr) };
            } else {
                i += 1;
            }
        }
    });
}

/// Agenda a destruição de um ponteiro.
pub fn defer_drop<T>(ptr: *mut T) {
    unsafe fn drop_ptr<T>(p: *mut ()) {
        let _ = Box::from_raw(p as *mut T);
    }

    let current_epoch = GLOBAL_EPOCH.load(Ordering::SeqCst);

    DEFER_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        queue.push((ptr as *mut (), drop_ptr::<T>, current_epoch));

        // Se a fila ficar grande, força uma tentativa de avanço.
        if queue.len() % 64 == 0 {
            drop(queue);
            try_advance();
            collect_garbage();
        }
    });
}

/// Um ponteiro atômico protegido por EBR.
pub struct AtomicEbr<T> {
    inner: AtomicPtr<T>,
}

impl<T> AtomicEbr<T> {
    pub fn new(val: T) -> Self {
        let ptr = Box::into_raw(Box::new(val));
        Self {
            inner: AtomicPtr::new(ptr),
        }
    }

    /// Carrega o ponteiro de forma segura, vinculando seu tempo de vida ao `Guard`.
    #[inline]
    pub fn load<'g>(&self, _guard: &'g Guard) -> Option<&'g T> {
        let ptr = self.inner.load(Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { &*ptr })
        }
    }

    /// Troca o ponteiro e defere a liberação do objeto antigo para o futuro.
    pub fn swap(&self, new_val: Option<T>, _guard: &Guard) {
        let new_ptr = match new_val {
            Some(val) => Box::into_raw(Box::new(val)),
            None => ptr::null_mut(),
        };

        let old_ptr = self.inner.swap(new_ptr, Ordering::SeqCst);
        if !old_ptr.is_null() {
            defer_drop(old_ptr);
        }
    }

    /// Agenda a liberação do objeto para o futuro (deferral).
    pub fn defer_destroy(&self, _guard: &Guard) {
        let ptr = self.inner.swap(ptr::null_mut(), Ordering::SeqCst);
        if !ptr.is_null() {
            defer_drop(ptr);
        }
    }
}

impl<T> Drop for AtomicEbr<T> {
    fn drop(&mut self) {
        let ptr = self.inner.load(Ordering::Relaxed);
        if !ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
    }
}
