package webjvm.io;

import java.io.IOException;
import java.io.OutputStream;

public class ConsoleOutputStream extends OutputStream {
    private boolean isError;

    public ConsoleOutputStream(boolean isError) {
        this.isError = isError;
    }

    @Override
    public void write(int b) {
        write0(b);
    }

    private native void write0(int b);
}
