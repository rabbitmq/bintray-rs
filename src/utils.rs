use serde_json;
use serde_json::value::Value;

#[derive(Deserialize)]
struct BintrayMessage {
    message: String,
}

#[derive(Deserialize)]
struct BintrayWarning {
    warn: String,
}

pub fn prettify_json(input: &str) -> String {
    match serde_json::from_str::<Value>(input) {
        Ok(json) => {
            match serde_json::to_string_pretty(&json) {
                Ok(output) => output,
                Err(_)     => String::from(input)
            }
        }
        Err(_) => String::from(input),
    }
}

#[macro_export]
macro_rules! format_status_line {
    ($resp:expr) => ({
        let status = $resp.status_raw();
        let status_code = status.0;
        let ref status_label = status.1;

        format!("{} {}", status_code, status_label)
    });
}

pub fn get_bintray_message(body: String, default_message: &str) -> String {
    match serde_json::from_str::<BintrayMessage>(&body) {
        Ok(message) => message.message,
        Err(_)      => default_message.to_string(),
    }
}

pub fn get_bintray_warning(body: &str) -> Option<String> {
    match serde_json::from_str::<BintrayWarning>(body) {
        Ok(message) => Some(message.warn),
        Err(_)      => None,
    }
}

#[macro_export]
macro_rules! report_bintray_warning {
    ($object:expr, $resp:expr, $body:expr, $function:expr) => ({
        let warning = utils::get_bintray_warning(&$body);

        match warning {
            Some(_) => {
                let status_line = format_status_line!($resp);
                warn!(
                    "{}({}): {}\n{}",
                    $function, $object, status_line, utils::prettify_json(&$body));
            }
            None => { }
        }

        Ok(warning)
    })
}

#[macro_export]
macro_rules! report_bintray_error {
    ($object:expr, $resp:expr, $body:expr, $function:expr, $errorkind:expr,
     $msg:expr) => ({
        let status_line = format_status_line!($resp);
        error!(
            "{}({}): {}\n{}",
            $function, $object, status_line, utils::prettify_json(&$body));

        let error = io::Error::new(
            $errorkind, format!(
                "Bintray::{}({}): {} ({})",
                $function, $object,
                utils::get_bintray_message($body, $msg),
                status_line));
        Err(BintrayError::from(error))
    });
    ($object:expr, $resp:expr, $body:expr, $function:expr, $errorkind:expr,
     $msg:expr, $log_as_info:expr) => ({
        let status_line = format_status_line!($resp);
        if $log_as_info {
            info!(
                "{}({}): {}\n{}",
                $function, $object, status_line, utils::prettify_json(&$body));
        } else {
            error!(
                "{}({}): {}\n{}",
                $function, $object, status_line, utils::prettify_json(&$body));
        }

        let error = io::Error::new(
            $errorkind, format!(
                "Bintray::{}({}): {} ({})",
                $function, $object,
                utils::get_bintray_message($body, $msg),
                status_line));
        Err(BintrayError::from(error))
    })
}
