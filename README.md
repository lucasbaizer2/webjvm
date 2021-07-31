# WebJVM

WebJVM is an implementation of the Java Virtual Machine, as defined by the [Java Virtual Machine Specification Java SE 8 Edition](https://docs.oracle.com/javase/specs/jvms/se8/html). Its internals are based upon OpenJDK, and the OpenJDK implementation of the Java Standard Library is used for WebJVM.

WebJVM is written in Rust and compiles to WebAssembly to execute Java applications within the context of a modern wen browser. WebJVM is intended to be able to run Java Applets, which are generally sandbox-escaping and potentially dangerous, inside the sandbox environment of a WebAssembly engine like [V8](https://v8.dev). This allows users to run programs (i.e. Java Applets) within a safe and secure context inside of their web browsers -- no downloads, no installers.

# Building

To build WebJVM for browsers, ensure you have Rust and [wasm-pack](https://rustwasm.github.io/wasm-pack) installed. Run `wasm-pack build -d test/pkg --target web --release` to build the WebAssembly archive.

Next, we need to get the Java SE 8 standard library. WebJVM does not provide an implementation of the standard library (known as `rt.jar` previous to Java 9), so we must add it to the classpath ourselves. WebJVM's native implementations of Java functions are based upon OpenJDK, so we need to use the `rt.jar` from OpenJDK 8. You can download OpenJDK 8 [here](https://adoptopenjdk.net). Take the `rt.jar` from the downloaded archive and place it into the `test/java` directory.

Once you've done that, start a local web server in the `test` directory. This can be anything, I personally use the Node.js package `http-server` to easily serve static content of a directory.  Once you have your server running, go to the root page in your (Wasm-capable, of course) web browser and open the console for output. If you want to modify the test class, edit `test/java/MainTest.java` and compile with the Java 8 compiler or earlier. Any compiler more recent than Java 8 will not work in the current state of WebJVM.

# License

WebJVM is under the [MIT License](https://github.com/lucasbaizer2/webjvm/blob/master/LICENSE). 
