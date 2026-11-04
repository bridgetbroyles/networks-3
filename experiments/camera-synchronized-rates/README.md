# S6 synchronized 1,000-byte camera-rate experiment

Truth Run 3 shows two distinct 1,000-byte sources at S6 with exactly the same
packet counts in every retained window: 194, 164, 210, 328, 328, 164, 328,
and 164. The sources move through four rate levels while remaining exactly
synchronized. Missing report intervals cannot create this equality because
both positive measurements are made locally before export.

The claim makes S6 the logging switch and emits source counters every 1.5
seconds. The detector requires two 1,000-byte sources with at least six aligned
windows, equality within one packet in every aligned window, at least three
distinct positive rate levels, and a largest level at least 1.7 times the
smallest. The variability rule rejects two merely constant equal-rate flows.

The adversarial audit includes a single source, constant twins, shifted burst
phases, unequal rates, one mismatched window, too few aligned windows, the same
behavior at another switch, wrong packet sizes, unrelated link plants,
malformed logs, and incomplete evidence.
