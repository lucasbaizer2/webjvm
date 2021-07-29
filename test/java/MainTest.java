public class MainTest {
    public static void main(String[] args) {
        long time = System.currentTimeMillis();
        System.out.println("Hello, new JVM!");
        long timeTook = System.currentTimeMillis() - time;
        System.out.println("Took " + timeTook + "ms to run a single print statement.");
    }
}
