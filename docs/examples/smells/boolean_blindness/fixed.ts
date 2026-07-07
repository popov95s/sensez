interface InvoiceDelivery {
  deliver(invoice: Invoice): void;
}

interface InvoiceArchival {
  archive(invoice: Invoice): void;
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

class PdfArchival implements InvoiceArchival {
  archive(invoice: Invoice): void {
    archive.store(renderPdf(invoice));
  }
}

class SkipArchival implements InvoiceArchival {
  archive(invoice: Invoice): void {
    return;
  }
}

function publishInvoice(
  invoiceId: string,
  delivery: InvoiceDelivery,
  archival: InvoiceArchival,
): void {
  const invoice = repo.load(invoiceId);
  delivery.deliver(invoice);
  archival.archive(invoice);
}
