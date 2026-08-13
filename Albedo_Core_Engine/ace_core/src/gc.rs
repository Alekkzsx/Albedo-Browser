// ============================================================================
// Albedo Core Engine (ACE)
// File: gc.rs
// Description: Fundamentos de Garbage Collection (Mark-and-Sweep).
//              Base para a futura Máquina Virtual JavaScript (Fase 7).
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::cell::Cell;
use std::ptr::NonNull;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcColor {
    Black, // Em uso (vivo)
    White, // Não visitado (candidato à coleta)
    Gray,  // Em processamento pelo Mark & Sweep
    Purple, // Suspeito de Ciclo (Refcount diminuiu mas não chegou a zero)
}

/// A Trait fundamental. Qualquer objeto que viva no Heap do GC e aponte para
/// outros objetos do GC deve implementar `Trace` para percorrer o grafo,
/// seja para Mark & Sweep ou para Cycle Collection.
pub trait Trace {
    /// O GC chamará este método para que o objeto marque seus filhos como Gray (ou para CCGC).
    fn trace(&self);
}

// ----------------------------------------------------------------------------
// GcBox (O Nó do Heap)
// ----------------------------------------------------------------------------

/// O cabeçalho escondido antes de cada alocação no GC.
pub struct GcHeader {
    color: Cell<GcColor>,
    ref_count: Cell<usize>, // CCGC: Contagem de referências para detecção de ciclos
    next: Option<NonNull<GcHeader>>,
    /// Um ponteiro de função para fazer o downcast do Drop e Trace.
    /// Isso é necessário porque o Heap guarda headers genéricos, mas precisa
    /// destruir os valores concretos T corretos na fase de Sweep.
    dropper: unsafe fn(*mut ()),
    tracer: unsafe fn(*mut ()),
}

use std::marker::PhantomData;

/// O Smart Pointer que o usuário interage. Funciona como um `Rc` ou `Box`,
/// mas a vida é gerenciada pelo Tri-Color Mark-and-Sweep.
pub struct GcBox<T: Trace + 'static> {
    ptr: NonNull<GcNode<T>>,
    _marker: PhantomData<*mut ()>, // Anula Send e Sync no Rust Stable
}

impl<T: Trace + 'static> Clone for GcBox<T> {
    fn clone(&self) -> Self {
        unsafe {
            let header = &self.ptr.as_ref().header;
            header.ref_count.set(header.ref_count.get() + 1);
        }
        Self {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
// (Copy removido porque GcBox agora tem Drop para decrementar ref_count)
    /// O Buffer de Suspeitos do Cycle Collector. Nós órfãos (ref_count diminuiu, mas > 0)
    /// são jogados aqui para serem rastreados por ciclos depois.
    pub static SUSPECT_BUFFER: std::cell::RefCell<Vec<NonNull<GcHeader>>> = std::cell::RefCell::new(Vec::new());
}

impl<T: Trace + 'static> Drop for GcBox<T> {
    fn drop(&mut self) {
        unsafe {
            let header = &self.ptr.as_ref().header;
            let count = header.ref_count.get();
            if count > 0 {
                header.ref_count.set(count - 1);
                // Se count - 1 for > 0, significa que nós o soltamos do JS, mas o DOM ainda aponta pra ele.
                // É um suspeito clássico de Ciclo.
                if count - 1 > 0 && header.color.get() != GcColor::Purple {
                    header.color.set(GcColor::Purple); // Pinta de suspeito
                    SUSPECT_BUFFER.with(|buf| {
                        buf.borrow_mut().push(self.ptr.cast());
                    });
                }
            }
        }
    }
}

#[repr(C)]
struct GcNode<T: Trace + 'static> {
    header: GcHeader,
    data: T,
}

impl<T: Trace + 'static> std::ops::Deref for GcBox<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().data }
    }
}

// ----------------------------------------------------------------------------
// GcHeap (O Colecionador)
// ----------------------------------------------------------------------------

pub struct GcHeap {
    head: Option<NonNull<GcHeader>>,
    bytes_allocated: usize,
    pub suspects: Vec<NonNull<GcHeader>>, // CCGC: Raízes suspeitas de ciclo
}

impl GcHeap {
    pub fn new() -> Self {
        Self {
            head: None,
            bytes_allocated: 0,
            suspects: Vec::new(),
        }
    }

    /// Aloca um novo valor no Heap gerenciado pelo GC.
    pub fn allocate<T: Trace + 'static>(&mut self, value: T) -> GcBox<T> {
        let node = Box::new(GcNode {
            header: GcHeader {
                color: Cell::new(GcColor::White),
                ref_count: Cell::new(1),
                next: self.head,
                dropper: drop_node::<T>,
                tracer: trace_node::<T>,
            },
            data: value,
        });

        let ptr = NonNull::from(Box::leak(node));
        self.head = Some(ptr.cast());
        self.bytes_allocated += std::mem::size_of::<GcNode<T>>();

        GcBox {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Executa o ciclo Tri-Color Mark-and-Sweep completo.
    /// Retorna quantos bytes foram coletados (Sweep).
    pub fn collect(&mut self, roots: &[&dyn Trace]) -> usize {
        // 1. MARK (Roots)
        for root in roots {
            root.trace();
        }

        // 2. SWEEP
        let mut bytes_freed = 0;
        let mut current = self.head;
        let mut prev: Option<NonNull<GcHeader>> = None;

        while let Some(mut node_ptr) = current {
            // SAFETY: O ponteiro é um nó válido alocado pelo GC e nós garantimos que a mutação `color` é síncrona.
            unsafe {
                let node = node_ptr.as_mut();
                if node.color.get() == GcColor::White {
                    // Unreachable! Coletar.
                    let next = node.next;
                    if let Some(mut p) = prev {
                        p.as_mut().next = next;
                    } else {
                        self.head = next;
                    }

                    // Dispara o Destructor customizado salvo no header
                    (node.dropper)(node_ptr.as_ptr() as *mut ());

                    current = next;
                    bytes_freed += 1; // Para simplicidade no retorno. No real, rastreamos size.
                } else {
                    // Sobrevivente: Reseta a cor para o próximo ciclo
                    node.color.set(GcColor::White);
                    prev = Some(node_ptr);
                    current = node.next;
                }
            }
        }

        bytes_freed
    }
}

// ----------------------------------------------------------------------------
// Funções Internas Type-Erased
// ----------------------------------------------------------------------------

// SAFETY: O ponteiro ptr é garantido pelo GC como pertencente a GcNode<T>. A deleção em tempo de execução
// faz cast de volta para o tipo correto e permite o Box desmantelar a estrutura.
unsafe fn drop_node<T: Trace + 'static>(ptr: *mut ()) {
    let typed_ptr = ptr as *mut GcNode<T>;
    // Recria o Box para o Rust invocar o Drop de `T` e liberar a memória do Heap
    let _ = Box::from_raw(typed_ptr);
}

// SAFETY: O ponteiro ptr é garantido como sendo do tipo GcNode<T>. Permite que o ciclo mark chame
// o trace original do objeto para continuar varrendo as raízes.
unsafe fn trace_node<T: Trace + 'static>(ptr: *mut ()) {
    let typed_ptr = ptr as *mut GcNode<T>;
    let node = &*typed_ptr;
    if node.header.color.get() == GcColor::White {
        node.header.color.set(GcColor::Black);
        node.data.trace();
    }
}

// A função que o usuário deve chamar de dentro dos seus `Trace` impls
pub fn mark<T: Trace + 'static>(gc_box: &GcBox<T>) {
    // SAFETY: Acessamos o tracer type-erased armazenado no cabeçalho do GcBox para prosseguir na varredura.
    unsafe {
        let node_ptr = gc_box.ptr.as_ptr();
        ((*node_ptr).header.tracer)(node_ptr as *mut ());
    }
}

impl Drop for GcHeap {
    fn drop(&mut self) {
        // Ao destruir o Heap inteiro, força a coleta de tudo
        let empty_roots: &[&dyn Trace] = &[];
        self.collect(empty_roots);
    }
}
