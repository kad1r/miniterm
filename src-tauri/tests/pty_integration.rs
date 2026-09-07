use miniterm_lib::pty::{SessionManager, SpawnOpts};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn interactive_shell() -> (String, Vec<String>) {
    if cfg!(windows) {
        (r"C:\Windows\System32\cmd.exe".to_string(), vec![])
    } else {
        ("/bin/sh".to_string(), vec!["-i".to_string()])
    }
}

fn temp_cwd() -> String {
    std::env::temp_dir().to_string_lossy().to_string()
}

/// `snapshot` içinde `needle` görünene kadar bekler.
fn wait_for(mgr: &SessionManager, id: u64, needle: &str, timeout: Duration) -> String {
    let start = Instant::now();
    loop {
        let text = String::from_utf8_lossy(&mgr.snapshot(id).unwrap()).to_string();
        if text.contains(needle) {
            return text;
        }
        if start.elapsed() > timeout {
            panic!("timed out waiting for {needle:?}; buffer was:\n{text}");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn spawns_a_shell_and_buffers_its_output() {
    let mgr = SessionManager::new();
    let (program, args) = interactive_shell();
    let id = mgr
        .spawn(SpawnOpts {
            cwd: temp_cwd(),
            program,
            args,
            initial_command: None,
            cols: 80,
            rows: 24,
        })
        .expect("spawn failed");

    mgr.write(id, b"echo hello_from_write\r").unwrap();
    wait_for(&mgr, id, "hello_from_write", Duration::from_secs(10));
    mgr.kill(id).unwrap();
}

#[test]
fn runs_the_initial_command_through_stdin_and_keeps_the_shell_alive() {
    let mgr = SessionManager::new();
    let (program, args) = interactive_shell();
    let id = mgr
        .spawn(SpawnOpts {
            cwd: temp_cwd(),
            program,
            args,
            initial_command: Some("echo marker_42".to_string()),
            cols: 80,
            rows: 24,
        })
        .expect("spawn failed");

    wait_for(&mgr, id, "marker_42", Duration::from_secs(10));

    // Asıl mesele: ilk komut bittikten sonra shell hâlâ komut alabiliyor olmalı.
    // `-Command` / `-c` ile başlatılsaydı burada süreç çoktan ölmüş olurdu.
    mgr.write(id, b"echo still_alive\r").unwrap();
    wait_for(&mgr, id, "still_alive", Duration::from_secs(10));
    mgr.kill(id).unwrap();
}

#[test]
fn reports_exit_with_a_status_code() {
    let mgr = SessionManager::new();
    let seen: Arc<Mutex<Vec<(u64, Option<i32>)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    mgr.set_exit_handler(move |id, code| sink.lock().unwrap().push((id, code)));

    let (program, args) = interactive_shell();
    let id = mgr
        .spawn(SpawnOpts {
            cwd: temp_cwd(),
            program,
            args,
            initial_command: Some("exit 3".to_string()),
            cols: 80,
            rows: 24,
        })
        .unwrap();

    let start = Instant::now();
    loop {
        if seen.lock().unwrap().iter().any(|(sid, _)| *sid == id) {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "no exit event arrived");
        std::thread::sleep(Duration::from_millis(25));
    }

    let events = seen.lock().unwrap().clone();
    let (_, code) = events.iter().find(|(sid, _)| *sid == id).unwrap();
    assert_eq!(*code, Some(3));
}

#[test]
fn unknown_session_ids_are_errors_not_panics() {
    let mgr = SessionManager::new();
    assert!(mgr.write(9999, b"x").is_err());
    assert!(mgr.resize(9999, 10, 10).is_err());
    assert!(mgr.kill(9999).is_err());
    assert!(mgr.snapshot(9999).is_err());
}

#[cfg(unix)]
#[test]
fn resize_reaches_the_child_process() {
    let mgr = SessionManager::new();
    let id = mgr
        .spawn(SpawnOpts {
            cwd: temp_cwd(),
            program: "/bin/sh".into(),
            args: vec!["-i".into()],
            initial_command: None,
            cols: 80,
            rows: 24,
        })
        .unwrap();

    mgr.resize(id, 132, 43).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    mgr.write(id, b"stty size\r").unwrap();
    wait_for(&mgr, id, "43 132", Duration::from_secs(10));
    mgr.kill(id).unwrap();
}
