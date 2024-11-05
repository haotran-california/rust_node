use std::process::{Command, Stdio};
use std::thread;
use std::time; 
use std::time::Duration;
use std::io::{self, Read};
use std::process::Child;
use std::io::{BufRead, BufReader};

fn start_node_server() -> Child {
    let server_process = Command::new("node")
        .arg("tests/fixtures/server.js") // Path to your server.js file
        .spawn()
        .expect("Failed to start Node.js server");

    // Wait for the server to start
    thread::sleep(time::Duration::from_secs(2));
    server_process
}

fn stop_node_server(mut server: Child) {
    server.kill().expect("Failed to stop Node.js server");
}

#[tokio::test]
async fn test_case_1() {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;
    use tokio::time::{timeout, Duration};

    let file_name = "tests/list/02.js";

    let mut child = Command::new("cargo")
        .args(&["run", "main", file_name])
        .env("RUST_STDIO_UNBUFFERED", "1")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start process");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let mut reader = BufReader::new(stdout).lines();

    let mut found = false;
    let duration = Duration::from_secs(5);

    // Use timeout to prevent hanging indefinitely
    if let Ok(_) = timeout(duration, async {
        while let Some(line) = reader.next_line().await.expect("Failed to read line") {
            println!("Output: {}", line);

            if line.contains("Hello World") {
                found = true;
                println!("Test passed: Output contains 'Hello World'");
                break;
            }
        }
    })
    .await
    {
        // Completed within timeout
    } else {
        println!("Test failed: Timeout reached without finding 'Hello World'");
    }

    // Ensure the child process is terminated
    let _ = child.kill().await;
    let _ = child.wait().await;

    assert!(
        found,
        "Expected output to contain 'Hello World', but it was not found."
    );
}




// #[test]
// fn run_js_tests_with_server() {
//     // Start the Node.js server
//     //let mut server_process = start_node_server();

//     // Path to your test directory
//     let test_dir = "tests/list";

//     // Iterate through all files in the test directory
//     let paths = fs::read_dir(test_dir).unwrap();
//     for path in paths {
//         let file_path = path.unwrap().path();
//         let file_name = file_path.to_str().unwrap();

//         // Run the command: `cargo run main <filename>`
//         let output = Command::new("cargo")
//             .args(&["run", "main", file_name])
//             .output()
//             .expect("Failed to execute process");

//         if !output.status.success() {
//             eprintln!(
//                 "Test failed for {}: {}",
//                 file_name,
//                 String::from_utf8_lossy(&output.stderr)
//             );
//             server_process.kill().expect("Failed to kill server process");
//             panic!("Test failed!");
//         } else {
//             println!(
//                 "Test passed for {}: {}",
//                 file_name,
//                 String::from_utf8_lossy(&output.stdout)
//             );
//         }
//     }

//     // Kill the Node.js server process
//     server_process.kill().expect("Failed to kill server process");
// }
