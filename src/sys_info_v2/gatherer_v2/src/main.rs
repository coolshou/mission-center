use std::error::Error;

use ::dbus::blocking::Connection;
use dbus_crossroads::{Context, Crossroads};

#[allow(unused_imports)]
use logging::{critical, debug, error, info, message, warning};
#[allow(unused_imports)]
use utils::arraystring::ToArrayStringLossy;

mod dbus;
mod logging;
mod platform;
mod utils;

// This is our "Hello" object that we are going to store inside the crossroads instance.
struct SystemStatistics {
    processes: std::collections::HashMap<u32, dbus::Process<'static>>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Let's start by starting up a connection to the session bus and request a name.
    let c = Connection::new_session()?;
    c.request_name("io.missioncenter.MissionCenter", true, true, false)?;

    // Create a new crossroads instance.
    // The instance is configured so that introspection and properties interfaces
    // are added by default on object path additions.
    let mut cr = Crossroads::new();

    // Let's build a new interface, which can be used for "Hello" objects.
    let iface_token = cr.register("io.missioncenter.MissionCenter", |builder| {
        // Let's add a method to the interface. We have the method name, followed by
        // names of input and output arguments (used for introspection). The closure then controls
        // the types of these arguments. The last argument to the closure is a tuple of the input arguments.
        builder.method(
            "Hello",
            (),
            ("reply",),
            move |ctx: &mut Context, hello: &mut SystemStatistics, (): ()| {
                // And here's what happens when the method is called.
                println!("Incoming hello call from {}!", name);
                hello.called_count += 1;
                let reply = format!(
                    "Hello {}! This API has been used {} times.",
                    name, hello.called_count
                );

                // Now call the function we got earlier to get a signal message.
                // The function takes all its arguments as the second parameter, so we must again
                // tuple our single argument into a single-tuple.
                let signal_msg = hello_happened(ctx.path(), &(name,));
                // The ctx parameter can be used to conveniently send extra messages.
                ctx.push_msg(signal_msg);

                // And the return value from the method call is a tuple of the output arguments.
                Ok((reply,))
            },
        );
    });

    // Let's add the "/hello" path, which implements the com.example.dbustest interface,
    // to the crossroads instance.
    cr.insert(
        "/hello",
        &[iface_token],
        SystemStatistics { called_count: 0 },
    );

    // Serve clients forever.
    cr.serve(&c)?;
    unreachable!()
}
