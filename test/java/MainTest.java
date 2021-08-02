public class MainTest {
    public static void main(String[] args) {
        long time = System.currentTimeMillis();
        System.out.println("Hello, new JVM!");
        long timeTook = System.currentTimeMillis() - time;
        time = System.currentTimeMillis();
        System.out.println("Took " + timeTook + "ms to run a single print statement.");
        timeTook = System.currentTimeMillis() - time;
        System.out.println("Took " + timeTook + "ms to run the previous print statement.");

        time = System.currentTimeMillis();
        for (int i = 0; i < 1_000_000; i++) {
        }
        timeTook = System.currentTimeMillis() - time;
        System.out.println("Took " + timeTook + "ms to count to one million.");

        new RuntimeException("printStackTrace()").printStackTrace();

        throw new RuntimeException("Exiting by throwing an exception.");
    }
}
