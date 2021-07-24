package webjvm.io;

import java.io.IOException;
import java.io.OutputStream;

public class ConsoleOutputStream extends OutputStream {
    @Override
    public void write(int b) {
        write0(b);
    }

    private native void write0(int b);
}
