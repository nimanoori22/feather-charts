//! Application-owned session/phase domain; no calendar or strike conversion.
use feather_charts::{
    api::{ChartBehavior, ChartError},
    model::{
        data_consumer::TimedData,
        data_layer::DataLayerError,
        ihorz_scale_behavior::{HorzScaleBehavior, HorzScaleItemConverter, TimeMark},
        localization_options::LocalizationOptions,
        tick_marks::TickMark,
        time_data::{TickMarkWeightValue, TimeScalePoint},
    },
};
use std::rc::Rc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Open,
    Auction,
    Close,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionPoint {
    pub session: u32,
    pub phase: Phase,
}
impl SessionPoint {
    pub fn at(index: usize) -> Self {
        Self {
            session: (index / 3 + 1) as u32,
            phase: [Phase::Open, Phase::Auction, Phase::Close][index % 3],
        }
    }
    pub fn rank(self) -> u64 {
        u64::from(self.session) * 3
            + match self.phase {
                Phase::Open => 0,
                Phase::Auction => 1,
                Phase::Close => 2,
            }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionKey(pub u64);
#[derive(Debug)]
pub struct SessionError(pub Rc<String>);
impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SessionError {}
// Neither behavior nor its owned options needs Clone.
pub struct Options {
    pub prefix: String,
    pub fast_append: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            prefix: "S".into(),
            fast_append: false,
        }
    }
}
pub struct Sessions {
    options: Options,
    #[cfg(test)]
    pub audit: Rc<Audit>,
}
#[cfg(test)]
#[derive(Default)]
pub struct Audit {
    pub calls: std::cell::RefCell<Vec<(&'static str, usize)>>,
    pub fail_preprocess: std::cell::RefCell<Option<SessionError>>,
}
impl Sessions {
    pub fn new(options: Options) -> Self {
        Self {
            options,
            #[cfg(test)]
            audit: Default::default(),
        }
    }
}
pub struct Converter;
impl HorzScaleItemConverter<SessionPoint, SessionKey> for Converter {
    type Error = SessionError;
    fn convert(&mut self, item: &SessionPoint) -> Result<SessionKey, SessionError> {
        if item.session == 0 {
            return Err(SessionError(Rc::new("session zero is invalid".into())));
        }
        Ok(SessionKey(item.rank()))
    }
}
impl HorzScaleBehavior for Sessions {
    type Item = SessionPoint;
    type InternalItem = SessionKey;
    type Key = u64;
    type CacheKey = u64;
    type Options = Options;
    type Converter = Converter;
    type Error = SessionError;
    fn options(&self) -> &Options {
        &self.options
    }
    fn set_options(&mut self, options: Options) {
        #[cfg(test)]
        self.audit.calls.borrow_mut().push(("options", 0));
        self.options = options;
    }
    fn update_scale_options(
        &mut self,
        _: &feather_charts::model::time_scale_options::HorzScaleOptions,
    ) {
        #[cfg(test)]
        self.audit.calls.borrow_mut().push(("scale", 0));
    }
    fn update_formatter(&mut self, _: &LocalizationOptions<SessionPoint>) {
        #[cfg(test)]
        self.audit.calls.borrow_mut().push(("formatter", 0));
    }
    fn preprocess_data<D: TimedData<Item = SessionPoint>>(
        &mut self,
        _: &mut [D],
    ) -> Result<(), SessionError> {
        #[cfg(test)]
        if let Some(error) = self.audit.fail_preprocess.borrow_mut().take() {
            return Err(error);
        }
        Ok(())
    }
    fn to_internal(&self, item: &SessionPoint) -> Result<SessionKey, SessionError> {
        Converter.convert(item)
    }
    fn create_converter_to_internal<D: TimedData<Item = SessionPoint>>(
        &mut self,
        _: &[D],
    ) -> Result<Converter, SessionError> {
        Ok(Converter)
    }
    fn key(&self, item: &SessionKey) -> u64 {
        item.0
    }
    fn cache_key(&self, item: &SessionKey) -> u64 {
        item.0
    }
    fn format_item(&self, item: &SessionKey) -> String {
        format!(
            "{}{} {:?}",
            self.options.prefix,
            item.0 / 3,
            [Phase::Open, Phase::Auction, Phase::Close][(item.0 % 3) as usize]
        )
    }
    fn format_tickmark(
        &self,
        mark: &TickMark<SessionKey, SessionPoint>,
        _: &LocalizationOptions<SessionPoint>,
    ) -> String {
        self.format_item(&mark.time)
    }
    fn max_tick_mark_weight(&self, marks: &[TimeMark]) -> TickMarkWeightValue {
        marks.iter().map(|m| m.weight).max().unwrap_or_default()
    }
    fn fill_weights_for_points(
        &mut self,
        points: &mut [TimeScalePoint<SessionKey, SessionPoint>],
        start: usize,
    ) {
        #[cfg(test)]
        self.audit
            .calls
            .borrow_mut()
            .push(("weights", points.len()));
        for point in &mut points[start..] {
            point.time_weight =
                TickMarkWeightValue::new(if point.time.0.is_multiple_of(3) { 7 } else { 1 });
        }
    }
    fn append_weight(
        &mut self,
        _: Option<&SessionKey>,
        current: &SessionKey,
    ) -> Option<TickMarkWeightValue> {
        #[cfg(test)]
        self.audit.calls.borrow_mut().push(("append", 0));
        self.options
            .fast_append
            .then(|| TickMarkWeightValue::new(if current.0.is_multiple_of(3) { 7 } else { 1 }))
    }
}
impl ChartBehavior for Sessions {
    fn conversion_error(error: SessionError) -> ChartError {
        ChartError::CustomBehavior(Box::new(error))
    }
    fn data_error(error: DataLayerError<SessionError>) -> ChartError {
        ChartError::CustomBehavior(Box::new(error))
    }
    fn validation_converter<D: TimedData<Item = SessionPoint>>(
        &self,
        _: &[D],
    ) -> Result<Converter, SessionError> {
        Ok(Converter)
    }
}
