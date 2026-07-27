abstract class Worker {
  abstract run(): void;
}

class FileWorker extends Worker {
  run() {}
  openFile() {}
  rotateFile() {}
}

class QueueWorker extends Worker {
  run() {}
  reserveJob() {}
  acknowledgeJob() {}
}
