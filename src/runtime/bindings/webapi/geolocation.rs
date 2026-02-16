use rquickjs::{Class, Ctx, Result, Value, Object, Function};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Geolocation {}

#[rquickjs::methods]
impl Geolocation {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[qjs(rename = "getCurrentPosition")]
    pub fn get_current_position(&self, success: Function<'_>, _error: Option<Function<'_>>, _options: Option<Object<'_>>) -> Result<()> {
        let ctx = success.ctx();
        let coords = rquickjs::Object::new(ctx.clone())?;
        // Stub coordinates (Google HQ)
        coords.set("latitude", 37.4220)?;
        coords.set("longitude", -122.0841)?;
        coords.set("accuracy", 10.0)?;
        
        let position = rquickjs::Object::new(ctx.clone())?;
        position.set("coords", coords)?;
        position.set("timestamp", chrono::Utc::now().timestamp_millis())?;
        
        let _ = success.call::<(&Object,), ()>((&position,));
        Ok(())
    }

    #[qjs(rename = "watchPosition")]
    pub fn watch_position(&self, success: Function<'_>, error: Option<Function<'_>>, options: Option<Object<'_>>) -> Result<u32> {
        self.get_current_position(success, error, options)?;
        Ok(1)
    }

    #[qjs(rename = "clearWatch")]
    pub fn clear_watch(&self, _id: u32) -> Result<()> {
        Ok(())
    }
}

pub fn register(ctx: &rquickjs::Context) -> Result<()> {
    ctx.with(|ctx| {
        Class::<Geolocation>::register(ctx.clone())?;
        Ok(())
    })
}
