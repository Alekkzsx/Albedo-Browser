use crate::ace::runtime::bindings::html::element::Element;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::cell::RefCell;

#[derive(Clone)]
#[rquickjs::class]
pub struct IntersectionObserver {
    rt: JsRuntime,
    callback: Persistent<Function<'static>>,
    thresholds_list: Vec<f32>,
    _observer_id: usize,
    // Targets registrados neste observer (node_idx)
    targets: RefCell<Vec<usize>>,
}

#[rquickjs::methods]
impl IntersectionObserver {
    #[qjs(constructor)]
    pub fn new<'js>(
        ctx: Ctx<'js>,
        callback: Function<'js>,
        options: Option<Object<'js>>,
    ) -> Result<Self> {
        let rt = ctx
            .globals()
            .get::<_, JsRuntime>("__albedo_rt__")
            .expect("JsRuntime required");

        // Extrair threshold(s) das opções
        let thresholds_list = if let Some(ref opts) = options {
            // threshold pode ser número ou array de números
            if let Ok(arr) = opts.get::<_, rquickjs::Array>("threshold") {
                let mut v = Vec::new();
                for i in 0..arr.len() {
                    if let Ok(t) = arr.get::<f32>(i) {
                        v.push(t);
                    }
                }
                if v.is_empty() {
                    vec![0.0]
                } else {
                    v
                }
            } else if let Ok(t) = opts.get::<_, f32>("threshold") {
                vec![t]
            } else {
                vec![0.0]
            }
        } else {
            vec![0.0]
        };

        // Gerar observer_id único via event_loop
        let observer_id = {
            let mut el = rt.event_loop.lock().unwrap();
            el.next_observer_id += 1;
            el.next_observer_id
        };

        let callback_persistent = Persistent::save(&ctx, callback);
        // SAFETY: The Persistent reference is valid for the lifetime of the observer.
        // QuickJS guarantees persistent references remain valid until explicitly dropped.
        let callback_stored: Persistent<Function<'static>> =
            unsafe { std::mem::transmute(callback_persistent) };

        Ok(Self {
            rt,
            callback: callback_stored,
            thresholds_list,
            _observer_id: observer_id,
            targets: RefCell::new(Vec::new()),
        })
    }

    pub fn observe<'js>(&self, _ctx: Ctx<'js>, target: Value<'js>) {
        // Extrair node_idx do Element JS
        let node_idx = Self::extract_node_idx(&target);

        if let Some(idx) = node_idx {
            // Evitar duplicatas
            let already = self.targets.borrow().contains(&idx);
            if !already {
                self.targets.borrow_mut().push(idx);

                // Registrar no intersection_registry do JsRuntime para cada threshold
                let mut registry = self.rt.intersection_registry.lock().unwrap();
                let obs_list = registry.entry(idx).or_insert_with(Vec::new);

                // Usar o menor threshold (ou 0.0 se lista vazia)
                let threshold = self.thresholds_list.first().copied().unwrap_or(0.0);

                // Restaurar callback para armazenar
                // Precisamos de um Persistent compartilhável — clonamos o stored
                let cb_clone = self.callback.clone();
                obs_list.push((cb_clone, threshold));
            }
        } else {
            // Fallback: usar id=0 como placeholder (comportamento anterior)
            self.targets.borrow_mut().push(0);
        }
    }

    pub fn unobserve<'js>(&self, _ctx: Ctx<'js>, target: Value<'js>) {
        if let Some(idx) = Self::extract_node_idx(&target) {
            self.targets.borrow_mut().retain(|&t| t != idx);

            // Remover do registry
            let mut registry = self.rt.intersection_registry.lock().unwrap();
            if let Some(obs_list) = registry.get_mut(&idx) {
                // Remover entradas com o mesmo observer_id
                // Usando tamanho da lista como proxy (simplificado: remove tudo para este nó)
                obs_list.clear();
            }
        }
    }

    pub fn disconnect(&self) {
        let targets: Vec<usize> = self.targets.borrow().clone();
        let mut registry = self.rt.intersection_registry.lock().unwrap();
        for idx in &targets {
            if let Some(obs_list) = registry.get_mut(idx) {
                obs_list.clear();
            }
        }
        self.targets.borrow_mut().clear();
    }

    #[qjs(get)]
    pub fn root(&self) -> Option<String> {
        None
    }

    #[qjs(get)]
    pub fn root_margin(&self) -> String {
        "0px".to_string()
    }

    #[qjs(get)]
    pub fn thresholds(&self) -> Vec<f32> {
        self.thresholds_list.clone()
    }
}

impl IntersectionObserver {
    /// Tenta extrair o node_idx de um Element JS via downcast.
    fn extract_node_idx(val: &Value<'_>) -> Option<usize> {
        if let Some(obj) = val.as_object() {
            // Tenta via Class instance downcast (caso mais comum — elemento nativo)
            if let Some(element_class) = Class::<Element>::from_object(&obj) {
                let element = element_class.borrow();
                return Some(element.index);
            }
            // Fallback: getter JS `node_idx` exposto pelo Element com #[qjs(get)]
            if let Ok(idx) = obj.get::<_, usize>("node_idx") {
                return Some(idx);
            }
        }
        None
    }
}

impl rquickjs::class::Trace<'_> for IntersectionObserver {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, '_>) {}
}
