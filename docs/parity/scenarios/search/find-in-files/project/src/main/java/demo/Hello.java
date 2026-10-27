package demo;

public class Hello {
    // greeting for the world
    public static String greeting(String name) {
        return "Hello, " + name + "!";
    }

    public static void main(String[] args) {
        System.out.println(greeting("a"));
        System.out.println(greeting("b"));
    }
}
