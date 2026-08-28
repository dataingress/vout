/// Do not use, use the rest of the loggers
#[macro_export]
macro_rules! raw_message_print {
    () => {};
    ($identifier:expr, $message:expr) => {
        let now = chrono::Utc::now().to_rfc3339();

        println!("{now}:{} :: {}", $identifier, $message);
    };
    ($identifier:expr, $message:expr, $($arg:ident : $value:expr),*) => {
        let now = chrono::Utc::now().to_rfc3339();
        let keys = vec![ $(format!("{}={:?}", stringify!($arg), $value)),* ];

        println!("{now}:{} :: {}; {}", $identifier, $message, keys.join(", "));
    };
}

#[macro_export]
macro_rules! outputln {
    () => {};
    ($message:expr) => {{
        use $crate::raw_message_print;
        raw_message_print!("INFO", $message);
    }};
    ($message:expr, $($arg:ident : $value:expr),*) => {{
        use $crate::raw_message_print;
        raw_message_print!("INFO", $message, $($arg : $value),*);
    }};
}

#[macro_export]
macro_rules! warnln {
    () => {};
    ($message:expr) => {{
        use $crate::raw_message_print;
        raw_message_print!("WARN", $message);
    }};
    ($message:expr, $($arg:ident : $value:expr),*) => {{
        use $crate::raw_message_print;
        raw_message_print!("WARN", $message, $($arg : $value),*);
    }};
}

#[macro_export]
macro_rules! errorln {
    () => {};
    ($message: expr) => {{
        use $crate::raw_message_print;
        raw_message_print!("ERRO", $message);
    }};
    ($message: expr, $($arg:ident : $value:expr),*) => {{
        use $crate::raw_message_print;
        raw_message_print!("ERRO", $message, $($arg : $value),*);
    }};
}

#[macro_export]
macro_rules! criticalln {
    () => {};
    ($message: expr) => {{
        use $crate::raw_message_print;
        raw_message_print!("CRIT", $message);
    }};
    ($message: expr, $($arg:ident : $value:expr),*) => {{
        use $crate::raw_message_print;
        raw_message_print!("CRIT", $message, $($arg : $value),*);
    }};
}
