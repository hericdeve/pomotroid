use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, Timelike};
use crate::db::{queries, DbState};
use crate::db::queries::ScheduledBlock;
use crate::google_calendar::api;

pub fn calculate_week_bounds(monday_ymd: &str) -> Result<(String, String, NaiveDate), String> {
    let monday = NaiveDate::parse_from_str(monday_ymd, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format (expected YYYY-MM-DD): {e}"))?;

    let monday_start = monday.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let sunday = monday + Duration::days(6);
    let sunday_end = sunday.and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap());

    let offset = *Local::now().offset();
    let time_min = DateTime::<Local>::from_naive_utc_and_offset(monday_start - offset, offset).to_rfc3339();
    let time_max = DateTime::<Local>::from_naive_utc_and_offset(sunday_end - offset, offset).to_rfc3339();

    Ok((time_min, time_max, monday))
}

pub fn current_monday_ymd() -> String {
    let today = Local::now().date_naive();
    let days_from_monday = today.weekday().num_days_from_monday();
    let monday = today - Duration::days(days_from_monday as i64);
    monday.format("%Y-%m-%d").to_string()
}

pub fn block_slot_to_rfc3339(
    monday: NaiveDate,
    day_of_week: i32,
    start_minute: i32,
    end_minute: i32,
) -> (String, String) {
    let offset = *Local::now().offset();
    let day_date = monday + Duration::days(day_of_week as i64);

    let s_h = (start_minute / 60) as u32;
    let s_m = (start_minute % 60) as u32;
    let start_naive = day_date.and_time(NaiveTime::from_hms_opt(s_h.min(23), s_m.min(59), 0).unwrap());
    let start_dt = DateTime::<Local>::from_naive_utc_and_offset(start_naive - offset, offset);

    // Handle blocks that spill past midnight (e.g. > 1440 mins)
    let end_days_add = (end_minute / 1440) as i64;
    let rem_end_min = end_minute % 1440;
    let e_h = (rem_end_min / 60) as u32;
    let e_m = (rem_end_min % 60) as u32;
    let end_date = day_date + Duration::days(end_days_add);
    let end_naive = end_date.and_time(NaiveTime::from_hms_opt(e_h.min(23), e_m.min(59), 0).unwrap());
    let end_dt = DateTime::<Local>::from_naive_utc_and_offset(end_naive - offset, offset);

    (start_dt.to_rfc3339(), end_dt.to_rfc3339())
}

pub async fn sync_calendar_two_way(
    db: &DbState,
    access_token: &str,
    calendar_id: &str,
    monday_ymd: &str,
) -> Result<Vec<ScheduledBlock>, String> {
    let (time_min, time_max, _monday) = calculate_week_bounds(monday_ymd)?;

    // 1. Fetch existing blocks from SQLite that belong to this synced calendar
    let local_synced_blocks = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let all_blocks = queries::schedule_get_all(&conn).map_err(|e| format!("DB query error: {e}"))?;
        all_blocks
            .into_iter()
            .filter(|b| b.calendar_type == "google" && b.google_calendar_id.as_deref() == Some(calendar_id))
            .collect::<Vec<_>>()
    };

    // 1b. Push any unlinked local blocks to Google Calendar
    for local_block in &local_synced_blocks {
        if local_block.google_event_id.is_none() {
            log::info!("[gcal] Pushing unlinked local block #{} ('{}') to Google Calendar", local_block.id, local_block.subject);
            if let Err(e) = push_block_create_to_google(db, access_token, calendar_id, local_block.id, monday_ymd).await {
                log::error!("[gcal] Failed to push existing unlinked block #{} to Google Calendar: {e}", local_block.id);
            }
        }
    }

    // Re-fetch local synced blocks after syncing any unlinked blocks
    let local_synced_blocks = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let all_blocks = queries::schedule_get_all(&conn).map_err(|e| format!("DB query error: {e}"))?;
        all_blocks
            .into_iter()
            .filter(|b| b.calendar_type == "google" && b.google_calendar_id.as_deref() == Some(calendar_id))
            .collect::<Vec<_>>()
    };

    // 2. Fetch events from Google Calendar (no lock held!)
    let google_events = api::fetch_events(access_token, calendar_id, &time_min, &time_max).await?;


    let mut seen_google_event_ids = std::collections::HashSet::new();

    // 3. Process events and write back to database
    {
        let conn = db.lock().map_err(|e| e.to_string())?;

        for event in &google_events {
            let effective_event_id = event
                .recurring_event_id
                .as_deref()
                .unwrap_or(&event.id);

            seen_google_event_ids.insert(effective_event_id.to_string());
            seen_google_event_ids.insert(event.id.clone());

            // We only map timed events to study blocks
            if let (Some(start_dt_obj), Some(end_dt_obj)) = (&event.start, &event.end) {
                if let (Some(ref start_str), Some(ref end_str)) = (&start_dt_obj.date_time, &end_dt_obj.date_time) {
                    if let (Ok(s_dt), Ok(e_dt)) = (
                        DateTime::parse_from_rfc3339(start_str),
                        DateTime::parse_from_rfc3339(end_str),
                    ) {
                        let s_local = s_dt.with_timezone(&Local);
                        let e_local = e_dt.with_timezone(&Local);

                        let day_of_week = s_local.weekday().num_days_from_monday() as i32;
                        let start_minute = (s_local.hour() * 60 + s_local.minute()) as i32;
                        let end_minute = (e_local.hour() * 60 + e_local.minute()) as i32;

                        let summary = event
                            .summary
                            .as_deref()
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or("Study Session");

                        // Check if block already exists locally by google_event_id (master or instance)
                        let existing_block = local_synced_blocks.iter().find(|b| {
                            b.google_event_id.as_deref() == Some(effective_event_id)
                                || b.google_event_id.as_deref() == Some(&event.id)
                        });

                        if let Some(existing) = existing_block {
                            // Ensure google_event_id points to effective (master) event id
                            if existing.google_event_id.as_deref() != Some(effective_event_id) {
                                let _ = queries::schedule_update_google_event_id(
                                    &conn,
                                    existing.id,
                                    calendar_id,
                                    effective_event_id,
                                );
                            }

                            // Check if name or hour slot changed
                            if existing.subject != summary
                                || existing.day_of_week != day_of_week
                                || existing.start_minute != start_minute
                                || existing.end_minute != end_minute
                            {
                                let _ = queries::schedule_update_block_name_and_slot(
                                    &conn,
                                    existing.id,
                                    summary,
                                    day_of_week,
                                    start_minute,
                                    end_minute,
                                );
                            }
                        } else {
                            // Insert new block from Google Calendar
                            let _ = queries::schedule_add_block_full(
                                &conn,
                                summary,
                                day_of_week,
                                start_minute,
                                end_minute,
                                None,
                                None,
                                None,
                                "google",
                                Some(calendar_id),
                                Some(effective_event_id),
                            );
                        }
                    }
                }
            }
        }

        // 4. Delete blocks from SQLite that were removed in Google Calendar
        for local_block in local_synced_blocks {
            if let Some(ref gid) = local_block.google_event_id {
                if !seen_google_event_ids.contains(gid) {
                    let _ = queries::schedule_delete_block(&conn, local_block.id);
                }
            }
        }
    }

    // Return all blocks
    let conn = db.lock().map_err(|e| e.to_string())?;
    queries::schedule_get_all(&conn).map_err(|e| format!("Failed to reload blocks: {e}"))
}

pub async fn push_block_create_to_google(
    db: &DbState,
    access_token: &str,
    calendar_id: &str,
    block_id: i64,
    monday_ymd: &str,
) -> Result<String, String> {
    let block = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        queries::schedule_get_by_id(&conn, block_id)
            .map_err(|e| format!("DB error: {e}"))?
            .ok_or_else(|| format!("Block #{block_id} not found"))?
    };

    let (_, _, monday) = calculate_week_bounds(monday_ymd)?;
    let (start_iso, end_iso) = block_slot_to_rfc3339(
        monday,
        block.day_of_week,
        block.start_minute,
        block.end_minute,
    );

    let event_id = api::create_event(access_token, calendar_id, &block.subject, &start_iso, &end_iso, None).await?;

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        queries::schedule_update_google_event_id(&conn, block_id, calendar_id, &event_id)
            .map_err(|e| format!("Failed to link block to Google event: {e}"))?;
    }

    Ok(event_id)
}

pub async fn push_block_update_to_google(
    db: &DbState,
    access_token: &str,
    block_id: i64,
    monday_ymd: &str,
) -> Result<(), String> {
    let block = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        queries::schedule_get_by_id(&conn, block_id)
            .map_err(|e| format!("DB error: {e}"))?
            .ok_or_else(|| format!("Block #{block_id} not found"))?
    };

    if let Some(ref cal_id) = block.google_calendar_id {
        if let Some(ref event_id) = block.google_event_id {
            let (_, _, monday) = calculate_week_bounds(monday_ymd)?;
            let (start_iso, end_iso) = block_slot_to_rfc3339(
                monday,
                block.day_of_week,
                block.start_minute,
                block.end_minute,
            );

            api::update_event(access_token, cal_id, event_id, &block.subject, &start_iso, &end_iso, None).await?;
        } else {
            push_block_create_to_google(db, access_token, cal_id, block_id, monday_ymd).await?;
        }
    }

    Ok(())
}

pub async fn push_block_delete_to_google(
    access_token: &str,
    calendar_id: &str,
    google_event_id: &str,
) -> Result<(), String> {
    api::delete_event(access_token, calendar_id, google_event_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_monday_ymd_is_monday() {
        let ymd = current_monday_ymd();
        let date = NaiveDate::parse_from_str(&ymd, "%Y-%m-%d").expect("valid date format");
        assert_eq!(date.weekday(), chrono::Weekday::Mon);
    }

    #[test]
    fn test_calculate_week_bounds() {
        let (time_min, time_max, monday) = calculate_week_bounds("2026-09-21").expect("bounds");
        assert_eq!(monday.weekday(), chrono::Weekday::Mon);
        assert!(time_min.contains("2026-09-21"));
        assert!(time_max.contains("2026-09-27"));
    }

    #[test]
    fn test_block_slot_to_rfc3339() {
        let monday = NaiveDate::from_ymd_opt(2026, 9, 21).unwrap();
        let (start, end) = block_slot_to_rfc3339(monday, 1, 600, 720); // Tuesday 10:00 to 12:00
        assert!(start.contains("2026-09-22T10:00:00"));
        assert!(end.contains("2026-09-22T12:00:00"));
    }

    #[test]
    fn test_local_timezone_is_valid() {
        let tz = api::get_local_timezone();
        assert!(!tz.is_empty());
    }
}

