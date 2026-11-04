# S9 Ledger pause experiment

This experiment tests the hypothesis that the Ledger-like 1,000-byte source
at S9 pauses for a long interval and then resumes. The claim observer writes
locally at S9, emits an explicit heartbeat and zero counter record every 1.5
seconds, and therefore distinguishes real source silence from lost reports.

The detector requires normal traffic before the silence, at least eleven
observed seconds of explicit zero windows, and later recovery. It also checks
the source's normal packet size and rate so that another paused customer is a
negative case.
