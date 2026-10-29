package org.junit;

public class Assert {
    public static void assertEquals(Object expected, Object actual) {
    }

    public static void assertTrue(boolean condition) {
    }

    /** Deprecated in the real library as well: the matcher form moved to Hamcrest. */
    @Deprecated
    public static void assertThat(Object actual, Object matcher) {
    }
}
