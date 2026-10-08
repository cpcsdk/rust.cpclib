from typing import Callable

# `on_output(kind, text)`: what a task or build reports while it runs. Kinds:
# "stdout", "stderr", "rule-start", "rule-stop", "rule-skipped", "rule-failed",
# "task-start", "task-stop", "task-ignored-error".
OutputCallback = Callable[[str, str], None]
