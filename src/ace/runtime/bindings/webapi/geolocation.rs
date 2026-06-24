use rquickjs::{Class, Ctx, Object, Result};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Geolocation {}

#[rquickjs::methods]
impl Geolocation {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    /// TODO: add docs
    pub fn get_current_position<'js>(
        &self,
        ctx: Ctx<'js>,
        success: rquickjs::Function<'js>,
        _error: Option<rquickjs::Function<'js>>,
    ) -> Result<()> {
        let position = rquickjs::Object::new(ctx.clone())?;
        let coords = rquickjs::Object::new(ctx.clone())?;
        coords.set("latitude", 0.0)?;
        coords.set("longitude", 0.0)?;
        position.set("coords", coords)?;

        let _ = success.call::<(Object,), ()>((position,));
        Ok(())
    }
}

/// TODO: add docs
pub fn register(ctx: &rquickjs::Context) -> Result<()> {
    ctx.with(|ctx| {
        let navigator = ctx.globals().get::<_, Object>("navigator")?;
        let geolocation = Class::instance(ctx.clone(), Geolocation {})?;
        navigator.set("geolocation", geolocation)?;
        Ok(())
    })
}
