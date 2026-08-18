use crate::domain::{RecurrenceFreq, RecurrenceSpec};
use block2::RcBlock;
use chrono::Weekday;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, ProtocolObject};
use objc2::{
    define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadOnly,
};
use objc2_event_kit::{
    EKAlarm, EKEvent, EKEventStore, EKRecurrenceDayOfWeek, EKRecurrenceEnd,
    EKRecurrenceFrequency, EKRecurrenceRule, EKSpan, EKWeekday,
};
use objc2_event_kit_ui::{
    EKEventViewAction, EKEventViewController, EKEventViewDelegate,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSCalendar, NSDate, NSDateComponents, NSError,
    NSNumber, NSObject, NSObjectProtocol, NSString,
};
use objc2_ui_kit::{
    UIApplication, UIBarButtonItem, UIBarButtonSystemItem,
    UINavigationController,
};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct EventRequest {
    pub title: String,
    pub notes: String,
    pub recurrence: Option<String>,
    pub recurrence_until: Option<(i32, i32, i32)>,
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub start_hour: i32,
    pub start_minute: i32,
    pub end_hour: i32,
    pub end_minute: i32,
    pub all_day: bool,
}

unsafe fn day_date(
    cal: &NSCalendar,
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
) -> Option<Retained<NSDate>> {
    let c = NSDateComponents::new();
    c.setYear(year as isize);
    c.setMonth(month as isize);
    c.setDay(day as isize);
    c.setHour(hour as isize);
    c.setMinute(minute as isize);
    cal.dateFromComponents(&c)
}

#[derive(Debug)]
pub enum ReminderOutcome {
    Created(String),
    AccessDenied,
    Failed(String),
}

async fn poll_flag(flag: Arc<Mutex<Option<bool>>>) -> Option<bool> {
    for _ in 0..600 {
        if let Some(granted) = *flag.lock().unwrap() {
            return Some(granted);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    None
}

async fn request_full_access_events(store: &EKEventStore) -> bool {
    let flag: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
    let flag_cb = flag.clone();
    let block = RcBlock::new(move |granted: Bool, _err: *mut NSError| {
        *flag_cb.lock().unwrap() = Some(granted.as_bool());
    });
    unsafe {
        store.requestFullAccessToEventsWithCompletion(
            &*block as *const _ as *mut _,
        );
    }
    poll_flag(flag).await.unwrap_or(false)
}

pub async fn create_event(req: EventRequest) -> ReminderOutcome {
    let store = unsafe { EKEventStore::new() };
    if !request_full_access_events(&store).await {
        return ReminderOutcome::AccessDenied;
    }

    unsafe {
        let calendar = match store.defaultCalendarForNewEvents() {
            Some(c) => c,
            None => {
                return ReminderOutcome::Failed(
                    "no default events calendar".into(),
                )
            }
        };

        let cal = NSCalendar::currentCalendar();

        let event = EKEvent::eventWithEventStore(&store);
        event.setTitle(Some(&NSString::from_str(&req.title)));
        if !req.notes.trim().is_empty() {
            event.setNotes(Some(&NSString::from_str(&req.notes)));
        }
        event.setCalendar(Some(&calendar));

        let alarm_date = if req.all_day {
            let day = match day_date(&cal, req.year, req.month, req.day, 0, 0) {
                Some(d) => d,
                None => return ReminderOutcome::Failed("bad date".into()),
            };
            event.setStartDate(Some(&day));
            event.setEndDate(Some(&day));
            event.setAllDay(true);
            day_date(
                &cal,
                req.year,
                req.month,
                req.day,
                req.start_hour,
                req.start_minute,
            )
        } else {
            let start = match day_date(
                &cal,
                req.year,
                req.month,
                req.day,
                req.start_hour,
                req.start_minute,
            ) {
                Some(d) => d,
                None => {
                    return ReminderOutcome::Failed("bad start date".into())
                }
            };
            let end = match day_date(
                &cal,
                req.year,
                req.month,
                req.day,
                req.end_hour,
                req.end_minute,
            ) {
                Some(d) => d,
                None => return ReminderOutcome::Failed("bad end date".into()),
            };
            event.setStartDate(Some(&start));
            event.setEndDate(Some(&end));
            Some(start)
        };

        if let Some(date) = alarm_date {
            let alarm = EKAlarm::alarmWithAbsoluteDate(&date);
            event.addAlarm(&alarm);
        }

        if let Some(spec) =
            req.recurrence.as_deref().and_then(RecurrenceSpec::parse)
        {
            // Recurrence is only set when the user gave an explicit end date, so the series is
            // always bounded; 23:59 keeps the last day's occurrence inside the range.
            let rec_end = req
                .recurrence_until
                .and_then(|(y, m, d)| day_date(&cal, y, m, d, 23, 59))
                .map(|date| EKRecurrenceEnd::recurrenceEndWithEndDate(&date));
            event.addRecurrenceRule(&build_recurrence_rule(
                &spec,
                rec_end.as_deref(),
            ));
        }

        match store.saveEvent_span_commit_error(&event, EKSpan::ThisEvent, true)
        {
            Ok(()) => ReminderOutcome::Created(
                event.calendarItemIdentifier().to_string(),
            ),
            Err(e) => {
                ReminderOutcome::Failed(e.localizedDescription().to_string())
            }
        }
    }
}

fn ek_weekday(w: Weekday) -> EKWeekday {
    match w {
        Weekday::Sun => EKWeekday::Sunday,
        Weekday::Mon => EKWeekday::Monday,
        Weekday::Tue => EKWeekday::Tuesday,
        Weekday::Wed => EKWeekday::Wednesday,
        Weekday::Thu => EKWeekday::Thursday,
        Weekday::Fri => EKWeekday::Friday,
        Weekday::Sat => EKWeekday::Saturday,
    }
}

unsafe fn build_recurrence_rule(
    spec: &RecurrenceSpec,
    end: Option<&EKRecurrenceEnd>,
) -> Retained<EKRecurrenceRule> {
    let freq = match spec.freq {
        RecurrenceFreq::Daily => EKRecurrenceFrequency::Daily,
        RecurrenceFreq::Weekly => EKRecurrenceFrequency::Weekly,
        RecurrenceFreq::Monthly => EKRecurrenceFrequency::Monthly,
    };
    let days_of_week = (!spec.weekdays.is_empty()).then(|| {
        let items: Vec<Retained<EKRecurrenceDayOfWeek>> = spec
            .weekdays
            .iter()
            .map(|w| EKRecurrenceDayOfWeek::dayOfWeek(ek_weekday(*w)))
            .collect();
        NSArray::from_retained_slice(&items)
    });
    let days_of_month = (!spec.month_days.is_empty()).then(|| {
        let items: Vec<Retained<NSNumber>> = spec
            .month_days
            .iter()
            .map(|d| NSNumber::new_isize(*d as isize))
            .collect();
        NSArray::from_retained_slice(&items)
    });
    EKRecurrenceRule::initRecurrenceWithFrequency_interval_daysOfTheWeek_daysOfTheMonth_monthsOfTheYear_weeksOfTheYear_daysOfTheYear_setPositions_end(
        EKRecurrenceRule::alloc(),
        freq,
        1,
        days_of_week.as_deref(),
        days_of_month.as_deref(),
        None,
        None,
        None,
        None,
        end,
    )
}

struct DoneDelegateIvars {
    nav: Retained<UINavigationController>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FlowFlowEventDoneDelegate"]
    #[ivars = DoneDelegateIvars]
    struct DoneDelegate;

    unsafe impl NSObjectProtocol for DoneDelegate {}

    unsafe impl EKEventViewDelegate for DoneDelegate {
        #[unsafe(method(eventViewController:didCompleteWithAction:))]
        fn did_complete(
            &self,
            _controller: &EKEventViewController,
            _action: EKEventViewAction,
        ) {
            self.dismiss();
        }
    }

    impl DoneDelegate {
        #[unsafe(method(done:))]
        fn done(&self, _sender: Option<&AnyObject>) {
            self.dismiss();
        }
    }
);

impl DoneDelegate {
    fn new(
        mtm: MainThreadMarker,
        nav: Retained<UINavigationController>,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(DoneDelegateIvars { nav });
        unsafe { msg_send![super(this), init] }
    }

    fn dismiss(&self) {
        let nav = &self.ivars().nav;
        if let Some(presenter) = nav.presentingViewController() {
            presenter.dismissViewControllerAnimated_completion(true, None);
        } else {
            nav.dismissViewControllerAnimated_completion(true, None);
        }
    }
}

thread_local! {
    static LIVE_DELEGATE: RefCell<Option<Retained<DoneDelegate>>> =
        const { RefCell::new(None) };
}

fn set_live_delegate(d: Retained<DoneDelegate>) {
    LIVE_DELEGATE.with(|slot| *slot.borrow_mut() = Some(d));
}

#[allow(deprecated)]
pub async fn present_event(identifier: String) {
    let store = unsafe { EKEventStore::new() };
    if !request_full_access_events(&store).await {
        eprintln!("[reminder] present_event: access denied");
        return;
    }
    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("[reminder] present_event: not on main thread");
        return;
    };
    unsafe {
        let id = NSString::from_str(&identifier);
        let Some(event) = store.eventWithIdentifier(&id) else {
            eprintln!(
                "[reminder] present_event: event not found id={identifier}"
            );
            return;
        };
        let evc = EKEventViewController::new(mtm);
        evc.setEvent(Some(&event));
        // Puts iOS's own Edit button in the sheet: the agent's generated title, the time and
        // the alert are all fixable there, so nothing has to be re-asked in the chat.
        evc.setAllowsEditing(true);
        let nav = UINavigationController::initWithRootViewController(
            UINavigationController::alloc(mtm),
            &evc,
        );
        let delegate = DoneDelegate::new(mtm, nav.clone());
        evc.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        let delegate_target: &AnyObject = &delegate;
        let done_item =
            UIBarButtonItem::initWithBarButtonSystemItem_target_action(
                UIBarButtonItem::alloc(mtm),
                UIBarButtonSystemItem::Done,
                Some(delegate_target),
                Some(sel!(done:)),
            );
        evc.navigationItem().setRightBarButtonItem(Some(&done_item));
        set_live_delegate(delegate);
        let app = UIApplication::sharedApplication(mtm);
        let Some(window) = app.keyWindow() else {
            eprintln!("[reminder] present_event: no key window");
            return;
        };
        let Some(root) = window.rootViewController() else {
            eprintln!("[reminder] present_event: no root vc");
            return;
        };
        root.presentViewController_animated_completion(&nav, true, None);
        eprintln!("[reminder] present_event: presented id={identifier}");
    }
}

pub async fn remove_event(identifier: String) -> Result<(), String> {
    let store = unsafe { EKEventStore::new() };
    if !request_full_access_events(&store).await {
        return Err("calendar access denied".into());
    }
    unsafe {
        let id = NSString::from_str(&identifier);
        let Some(event) = store.eventWithIdentifier(&id) else {
            return Err("event not found".into());
        };
        store
            .removeEvent_span_commit_error(&event, EKSpan::ThisEvent, true)
            .map_err(|e| e.localizedDescription().to_string())
    }
}
