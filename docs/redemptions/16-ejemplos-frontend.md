# 16 - Ejemplos Frontend (JavaScript/React)

## React Hook para Redenciones

```javascript
import { useState } from 'react';

export function useRedemptions() {
  const [loading, setLoading] = useState(false);
  
  const createRedemption = async (offerId) => {
    setLoading(true);
    try {
      const response = await fetch('/api/v1/rewards/redeem', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('token')}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          offer_id: offerId,
          user_id: getCurrentUserId()
        })
      });
      
      const data = await response.json();
      return data;
    } finally {
      setLoading(false);
    }
  };
  
  return { createRedemption, loading };
}
```

## Componente de Ofertas

```jsx
function OffersList() {
  const [offers, setOffers] = useState([]);
  
  useEffect(() => {
    fetch('/api/v1/rewards/offers', {
      headers: {
        'Authorization': `Bearer ${token}`
      }
    })
    .then(r => r.json())
    .then(data => setOffers(data.offers));
  }, []);
  
  return (
    <div>
      {offers.map(offer => (
        <OfferCard key={offer.offer_id} offer={offer} />
      ))}
    </div>
  );
}
```

**Siguiente**: [17-ejemplos-postman.md](./17-ejemplos-postman.md)
