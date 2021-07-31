# WebJVM

WebJVM is an implementation of the Java Virtual Machine, as defined by the [Java Virtual Machine Specification Java SE 8 Edition](https://docs.oracle.com/javase/specs/jvms/se8/html).

WebJVM is written in Rust and compiles to WebAssembly to execute Java applications within the context of a modern wen browser. WebJVM is intended to be able to run Java Applets, which are generally sandbox-escaping and potentially dangerous, inside the sandbox environment of a WebAssembly engine like [V8](https://v8.dev). This allows users to run programs (i.e. Java Applets) within a safe and secure context inside of their web browsers -- no downloads, no installers.

# License

WebJVM is under the [MIT License](https://github.com/lucasbaizer2/webjvm/blob/master/LICENSE). 
