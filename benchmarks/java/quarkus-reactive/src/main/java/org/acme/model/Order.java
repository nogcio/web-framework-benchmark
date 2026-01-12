package org.acme.model;

import java.util.List;

public class Order {
    public String status;
    public long amount;
    public String country;
    public List<OrderItem> items;
}
