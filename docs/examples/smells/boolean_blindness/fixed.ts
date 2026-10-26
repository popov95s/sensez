interface InvoiceDelivery {
  deliver(invoice: Invoice): void;
}

interface InvoiceFollowUp {
  run(invoice: Invoice): void;
}

class EmailDelivery implements InvoiceDelivery {
  deliver(invoice: Invoice): void {
    mailer.send(invoice);
  }
}

class ReviewQueueDelivery implements InvoiceDelivery {
  deliver(invoice: Invoice): void {
    reviewQueue.add(invoice);
  }
}

class PdfArchival implements InvoiceFollowUp {
  run(invoice: Invoice): void {
    archive.store(renderPdf(invoice));
  }
}

class OwnerNotification implements InvoiceFollowUp {
  run(invoice: Invoice): void {
    notifier.ping(invoice.owner);
  }
}

function publishInvoice(
  invoiceId: string,
  delivery: InvoiceDelivery,
  followUps: InvoiceFollowUp[],
): void {
  const invoice = repo.load(invoiceId);
  delivery.deliver(invoice);
  for (const followUp of followUps) {
    followUp.run(invoice);
  }
}
