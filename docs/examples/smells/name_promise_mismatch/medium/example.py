class TaskQueue:
    def __init__(self) -> None:
        self.tasks: list[Task] = []

    def get_next_task(self) -> Task | None:
        next_task = self.tasks[0] if self.tasks else None
        self.tasks.remove(next_task)
        return next_task
