import java.util.*;
import java.security.*;
import javax.crypto.*;

public class MainTest {
    public static void main(String[] args) {
        byte[] nonce = new byte[16];

        SecureRandom random = new SecureRandom();
        random.nextBytes(nonce);

        System.out.println(Arrays.toString(nonce));
    }
}
