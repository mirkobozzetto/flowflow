use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_event_kit::{EKAlarm, EKEventStore, EKReminder};
use objc2_foundation::{NSCalendar, NSDateComponents, NSError, NSString};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ReminderRequest {
    pub title: String,
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub hour: i32,
    pub minute: i32,
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

async fn request_full_access(store: &EKEventStore) -> bool {
    let flag: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
    let flag_cb = flag.clone();
    let block = RcBlock::new(move |granted: Bool, _err: *mut NSError| {
        *flag_cb.lock().unwrap() = Some(granted.as_bool());
    });
    unsafe {
        store.requestFullAccessToRemindersWithCompletion(
            &*block as *const _ as *mut _,
        );
    }
    poll_flag(flag).await.unwrap_or(false)
}

pub async fn create_reminder(req: ReminderRequest) -> ReminderOutcome {
    let store = unsafe { EKEventStore::new() };
    if !request_full_access(&store).await {
        return ReminderOutcome::AccessDenied;
    }

    unsafe {
        let calendar = match store.defaultCalendarForNewReminders() {
            Some(c) => c,
            None => {
                return ReminderOutcome::Failed(
                    "no default reminders calendar".into(),
                )
            }
        };

        let comps = NSDateComponents::new();
        comps.setYear(req.year as isize);
        comps.setMonth(req.month as isize);
        comps.setDay(req.day as isize);
        comps.setHour(req.hour as isize);
        comps.setMinute(req.minute as isize);

        let reminder = EKReminder::reminderWithEventStore(&store);
        reminder.setTitle(Some(&NSString::from_str(&req.title)));
        reminder.setCalendar(Some(&calendar));
        reminder.setStartDateComponents(Some(&comps));
        reminder.setDueDateComponents(Some(&comps));

        let cal = NSCalendar::currentCalendar();
        if let Some(date) = cal.dateFromComponents(&comps) {
            let alarm = EKAlarm::alarmWithAbsoluteDate(&date);
            reminder.addAlarm(&alarm);
        }

        match store.saveReminder_commit_error(&reminder, true) {
            Ok(()) => ReminderOutcome::Created(
                reminder.calendarItemIdentifier().to_string(),
            ),
            Err(e) => {
                ReminderOutcome::Failed(e.localizedDescription().to_string())
            }
        }
    }
}

pub async fn remove_reminder(identifier: String) -> Result<(), String> {
    let store = unsafe { EKEventStore::new() };
    if !request_full_access(&store).await {
        return Err("reminders access denied".into());
    }
    unsafe {
        let id = NSString::from_str(&identifier);
        match store.calendarItemWithIdentifier(&id) {
            Some(item) => match item.downcast::<EKReminder>() {
                Ok(rem) => {
                    let r = store
                        .removeReminder_commit_error(&rem, true)
                        .map_err(|e| e.localizedDescription().to_string());
                    match &r {
                        Ok(()) => {
                            eprintln!("[reminder] revoked id={identifier}")
                        }
                        Err(e) => eprintln!(
                            "[reminder] revoke remove failed id={identifier}: {e}"
                        ),
                    }
                    r
                }
                Err(_) => {
                    eprintln!(
                        "[reminder] revoke: id={identifier} not a reminder"
                    );
                    Err("identifier is not a reminder".into())
                }
            },
            None => {
                eprintln!(
                    "[reminder] revoke: identifier not found id={identifier}"
                );
                Err("reminder not found by identifier".into())
            }
        }
    }
}
