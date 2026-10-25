export class TaskQueue {
  private tasks: Task[] = [];

  getNextTask(): Task | undefined {
    const next = this.tasks[0];
    this.tasks.splice(0, 1);
    return next;
  }
}
