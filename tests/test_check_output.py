import importlib.util
from pathlib import Path
import tempfile
import unittest

MODULE_PATH = Path(__file__).resolve().parents[1] / "scripts" / "check_output.py"
spec = importlib.util.spec_from_file_location("check_output", MODULE_PATH)
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)
PREFIX = b"314159265358979323846264338327950288419716939937510"
SYNTHETIC_DIGITS = PREFIX + b"0" * (1001 - len(PREFIX))


def pi_output(digits=SYNTHETIC_DIGITS, newline=b"\r\n"):
    text = b"pi ~= " + digits[:1] + b"." + digits[1:]
    return newline.join(text[i:i + 134] for i in range(0, len(text), 134)) + newline


def squares_output(newline=b"\r\n"):
    return b"".join(f"{i}\t{i*i}\t{2*i-1}".encode() + newline for i in range(1, 101))


def batch_output(newline=b"\r\n"):
    return b"Hello world from Test OS!" + newline + squares_output(newline) + pi_output(newline=newline) + b"primecheck: Found another 1000 primes; last one was 7919!" + newline


class OutputOracleTests(unittest.TestCase):
    def test_complete_batch(self):
        report = checker.check_output(batch_output(), SYNTHETIC_DIGITS)
        self.assertEqual(report, {"mode": "batch", "squares_rows": 100, "pi_digits": 1001, "first_1000th_prime": 7919})

    def test_capture_with_lf_line_endings(self):
        checker.check_output(batch_output(b"\n"), SYNTHETIC_DIGITS)

    def test_optional_kernel_pi_before_squares(self):
        data = batch_output().replace(b"1\t1\t1\r\n", pi_output() + b"1\t1\t1\r\n", 1)
        checker.check_output(data, SYNTHETIC_DIGITS)

    def test_pi_only(self):
        self.assertEqual(checker.check_output(pi_output(), SYNTHETIC_DIGITS, "pi")["pi_digits"], 1001)

    def test_other_os_name(self):
        checker.check_output(batch_output().replace(b"Test OS", b"Teo's kernel"), SYNTHETIC_DIGITS)

    def test_preamble_and_later_milestones(self):
        checker.check_output(b"build output\r\n" + batch_output() + b"primecheck: Found another 1000 primes; last one was 17389!\r\n", SYNTHETIC_DIGITS)

    def test_each_square_row_is_checked(self):
        for i in range(1, 101):
            with self.subTest(row=i):
                row = f"{i}\t{i*i}\t{2*i-1}\r\n".encode()
                bad = f"{i}\t{i*i+1}\t{2*i-1}\r\n".encode()
                with self.assertRaises(ValueError):
                    checker.check_output(batch_output().replace(row, bad, 1), SYNTHETIC_DIGITS)

    def test_first_difference_is_one_not_handout_typo(self):
        with self.assertRaises(ValueError):
            checker.check_output(batch_output().replace(b"1\t1\t1\r\n", b"1\t1\t0\r\n", 1), SYNTHETIC_DIGITS)

    def test_missing_row(self):
        with self.assertRaises(ValueError):
            checker.check_output(batch_output().replace(b"50\t2500\t99\r\n", b""), SYNTHETIC_DIGITS)

    def test_duplicate_row(self):
        with self.assertRaises(ValueError):
            checker.check_output(batch_output().replace(b"50\t2500\t99\r\n", b"50\t2500\t99\r\n" * 2), SYNTHETIC_DIGITS)

    def test_duplicate_greeting(self):
        with self.assertRaises(checker.IncorrectOutput):
            checker.check_output(b"Hello world from Extra core!\r\n" + batch_output(), SYNTHETIC_DIGITS)

    def test_wrong_prime_milestone(self):
        with self.assertRaises(checker.IncorrectOutput):
            checker.check_output(batch_output().replace(b"7919", b"7920"), SYNTHETIC_DIGITS)

    def test_wrong_order(self):
        with self.assertRaises(ValueError):
            checker.check_output(b"Hello world from Test OS!\r\n" + pi_output() + squares_output() + b"primecheck: Found another 1000 primes; last one was 7919!\r\n", SYNTHETIC_DIGITS)

    def test_every_pi_digit_is_checked(self):
        for index in (0, 1, 50, 133, 500, 999, 1000):
            with self.subTest(index=index):
                bad = bytearray(SYNTHETIC_DIGITS)
                bad[index] = 48 + (bad[index] - 48 + 1) % 10
                with self.assertRaises(checker.IncorrectOutput):
                    checker.check_output(pi_output(bytes(bad)), SYNTHETIC_DIGITS, "pi")

    def test_pi_truncated_before_other_output(self):
        with self.assertRaises(checker.IncorrectOutput):
            checker.check_output(pi_output(SYNTHETIC_DIGITS[:-1]) + b"done\r\n", SYNTHETIC_DIGITS, "pi")

    def test_extra_pi_digit(self):
        with self.assertRaises(checker.IncorrectOutput):
            checker.check_output(pi_output(SYNTHETIC_DIGITS + b"0"), SYNTHETIC_DIGITS, "pi")

    def test_pi_requires_line_terminator(self):
        with self.assertRaises(checker.IncompleteOutput):
            checker.check_output(pi_output().rstrip(b"\r\n"), SYNTHETIC_DIGITS, "pi")

    def test_incomplete_captures_do_not_pass(self):
        data = batch_output()
        for end in (0, 10, 30, 150, len(data) // 2, len(data) - 2):
            with self.subTest(end=end), self.assertRaises(ValueError):
                checker.check_output(data[:end], SYNTHETIC_DIGITS)

    def test_fixture_validation(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.txt"
            path.write_bytes(SYNTHETIC_DIGITS + b"\n")
            self.assertEqual(checker.read_pi_fixture(path), SYNTHETIC_DIGITS)
            for invalid in (b"", b"314159", b"x" * 1001, SYNTHETIC_DIGITS + b"0", b"0" * 1001):
                path.write_bytes(invalid)
                with self.assertRaises(ValueError):
                    checker.read_pi_fixture(path)

    def test_unknown_mode_is_rejected(self):
        with self.assertRaises(ValueError):
            checker.check_output(batch_output(), SYNTHETIC_DIGITS, "unknown")


if __name__ == "__main__":
    unittest.main()
